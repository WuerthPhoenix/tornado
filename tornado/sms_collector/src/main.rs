use chrono::NaiveDateTime;
use clap::Parser;
use config_rs::{Config, ConfigError, File};
use log::*;
use opentelemetry::trace::SpanKind;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tornado_common::actors::nats_publisher::NatsPublisherConfig;
use tornado_common_api::{Event, Value};
use tornado_common_logger::elastic_apm::DEFAULT_APM_SERVER_CREDENTIALS_FILENAME;
use tornado_common_logger::opentelemetry_logger::TelemetryContextInjector;
use tornado_common_logger::{setup_logger, LoggerConfig};
use tornado_common_metrics::opentelemetry::sdk::propagation::TraceContextPropagator;
use tracing_opentelemetry::OpenTelemetrySpanExt;

const SMS_EVENT_HANDLE_ACTION: &str = "RECEIVED";

type SmsFile = String;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SmsCollectorConfig {
    pub failed_sms_folder: String,
    pub tornado_connection_channel: NatsPublisherConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CollectorConfig {
    /// The logger configuration
    pub logger: LoggerConfig,
    pub sms_collector: SmsCollectorConfig,
}

#[derive(Parser)]
enum SmsCollectorAction {
    #[clap(name = "RECEIVED")]
    Received {
        #[clap(index = 1)]
        sms_file: SmsFile,
    },
}

#[derive(Parser)]
struct SmsCollectorArgs {
    #[clap(short = 'c', long = "config-dir")]
    config_dir: String,
    #[clap(index = 1)]
    event: String,
    #[clap(index = 2)]
    sms_file: SmsFile,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SmsEventPayload {
    #[serde(alias = "From")]
    sender: String,
    #[serde(alias = "Sent", deserialize_with = "deserialize_timestamp_from_datetime_string")]
    timestamp: i64,
    text: String,
}

#[derive(Error, Debug)]
enum SmsCollectorError {
    #[error("{0}")]
    ArgumentParseError(#[from] clap::Error),
    #[error("Could not load config file for the collector: {error}")]
    ConfigError {
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
        event: String,
        sms_file: SmsFile,
    },
    #[error("Ignoring event \"{event}\"")]
    IgnoreAction { event: String, sms_file: SmsFile },
    #[error("Could not find or open sms file {sms_file}: {error} ")]
    SmsFileAccessError { sms_file: SmsFile, error: io::Error },
    #[error("Could not forward sms contents to nats. The sms will be copied to {failed_sms_file}")]
    TornadoConnectionError { error: io::Error, sms_file: SmsFile, failed_sms_file: PathBuf },
}

impl SmsCollectorError {
    fn sms_file(&self) -> Option<&SmsFile> {
        match self {
            SmsCollectorError::ArgumentParseError(_) => None,
            SmsCollectorError::IgnoreAction { sms_file, .. } => Some(sms_file),
            SmsCollectorError::SmsFileAccessError { sms_file, .. } => Some(sms_file),
            SmsCollectorError::TornadoConnectionError { sms_file, .. } => Some(sms_file),
            SmsCollectorError::ConfigError { sms_file, .. } => Some(sms_file),
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let collector_result = execute_collector().await;

    match collector_result {
        Ok(sms_file) => {
            tokio::fs::remove_file(&sms_file).await?;
        }
        Err(err) => {
            error!("Could not forward the sms to Tornado. Error: {err}");

            match err {
                SmsCollectorError::TornadoConnectionError { sms_file, failed_sms_file, .. } => {
                    std::fs::rename(sms_file, failed_sms_file)?;
                }
                err => {
                    if let Some(sms_file) = err.sms_file() {
                        std::fs::remove_file(sms_file)?;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn execute_collector() -> Result<SmsFile, SmsCollectorError> {
    let args = SmsCollectorArgs::parse();

    let mut collector_config = match build_config(&args.config_dir) {
        Ok(config) => config,
        Err(err) => {
            return Err(SmsCollectorError::ConfigError {
                error: err.into(),
                event: args.event,
                sms_file: args.sms_file,
            })
        }
    };
    match setup_logger(collector_config.logger.clone()) {
        Ok(guard) => std::mem::forget(guard),
        Err(err) => {
            return Err(SmsCollectorError::ConfigError {
                error: err.into(),
                event: args.event,
                sms_file: args.sms_file,
            })
        }
    }

    if args.event != SMS_EVENT_HANDLE_ACTION {
        return Err(SmsCollectorError::IgnoreAction { event: args.event, sms_file: args.sms_file });
    }

    let apm_server_api_credentials_filepath =
        format!("{}/{}", args.config_dir, DEFAULT_APM_SERVER_CREDENTIALS_FILENAME);
    let apm_credentials_read_result = collector_config
        .logger
        .tracing_elastic_apm
        .read_apm_server_api_credentials_if_not_set(&apm_server_api_credentials_filepath);

    if let Err(apm_credentials_read_error) = apm_credentials_read_result {
        warn!("{:?}", apm_credentials_read_error);
    }

    let sms_content = match std::fs::read_to_string(&args.sms_file) {
        Ok(sms_content) => sms_content,
        Err(err) => {
            return Err(SmsCollectorError::SmsFileAccessError {
                sms_file: args.sms_file,
                error: err,
            });
        }
    };
    let sms_event_payload = parse_sms(&sms_content);

    let Value::Object(payload) = serde_json::to_value(sms_event_payload).unwrap() else { unreachable!() };

    let mut full_event_message = Event::new_with_payload("sms", payload);

    let span = tracing::info_span!("Received SMS", trace_id = tracing::field::Empty, 
            otel.kind = %SpanKind::Producer,
            peer.service = "tornado")
    .entered();
    let trace_id = full_event_message.get_trace_id_for_logging(&span.context());
    span.record("trace_id", &trace_id.as_ref());
    let trace_context_propagator = TraceContextPropagator::new();
    let trace_context =
        TelemetryContextInjector::get_trace_context_map(&span.context(), &trace_context_propagator);
    full_event_message.set_trace_context(trace_context);

    let serialized_full_event_message = serde_json::to_string(&full_event_message).unwrap();
    let nats = collector_config.sms_collector.tornado_connection_channel;
    match publish_on_nats(serialized_full_event_message, &nats).await {
        Ok(_) => Ok(args.sms_file),
        Err(err) => {
            let mut failed_sms_file =
                PathBuf::from(collector_config.sms_collector.failed_sms_folder);
            let sms_file = Path::new(&args.sms_file)
                .file_name()
                .expect("filename of already read file to be present.");
            failed_sms_file.push(sms_file);

            Err(SmsCollectorError::TornadoConnectionError {
                error: err,
                sms_file: args.sms_file,
                failed_sms_file,
            })
        }
    }
}

async fn publish_on_nats(
    serialized_full_event_message: String,
    nats: &NatsPublisherConfig,
) -> io::Result<()> {
    info!("Connect to Tornado through NATS");
    let nats_connection = nats.client.new_client().await?;
    nats_connection.publish(&nats.subject, serialized_full_event_message).await?;
    Ok(())
}

pub fn build_config(config_dir: &str) -> Result<CollectorConfig, ConfigError> {
    let config_file_path = format!("{}/{}", &config_dir, "sms_collector.toml");
    let mut s = Config::new();
    s.merge(File::with_name(&config_file_path))?;
    s.try_into()
}

pub fn parse_sms(sms: &str) -> Option<SmsEventPayload> {
    let (headers, text) = sms.split_once("\n\n")?;
    let mut headers: HashMap<_, _> = headers
        .lines()
        .flat_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim(), value.trim()))
        .collect();

    headers.insert("text", text.trim());
    let value = serde_json::to_value(headers).unwrap();
    let value = serde_json::from_value(value).unwrap();
    Some(value)
}

pub fn deserialize_timestamp_from_datetime_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    struct DateTimeVisitor;
    impl Visitor<'_> for DateTimeVisitor {
        type Value = i64;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("Sent datetime not formatted as expected: dd-mm-yy HH:MM:SS")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match NaiveDateTime::parse_from_str(v, "%y-%m-%d %H:%M:%S") {
                Ok(value) => Ok(value.timestamp()),
                Err(err) => Err(serde::de::Error::custom(err)),
            }
        }
    }

    deserializer.deserialize_str(DateTimeVisitor)
}
