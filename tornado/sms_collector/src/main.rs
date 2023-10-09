use chrono::NaiveDateTime;
use clap::Parser;
use config_rs::{Config, ConfigError, File};
use log::*;
use opentelemetry::trace::SpanKind;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::path::Path;
use tornado_common::actors::nats_publisher::NatsPublisherConfig;
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common_api::{Event, Value};
use tornado_common_logger::elastic_apm::DEFAULT_APM_SERVER_CREDENTIALS_FILENAME;
use tornado_common_logger::opentelemetry_logger::TelemetryContextInjector;
use tornado_common_logger::{setup_logger, LoggerConfig};
use tornado_common_metrics::opentelemetry::sdk::propagation::TraceContextPropagator;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct SmsCollectorConfig {
    pub failed_sms_folder: String,
    pub tornado_connection_channel: TornadoConnectionChannel,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CollectorConfig {
    /// The logger configuration
    pub logger: LoggerConfig,
    pub sms_collector: SmsCollectorConfig,
}

#[derive(Parser)]
enum SmsCollectorArgs {
    #[clap(name = "RECEIVED")]
    Received {
        #[clap(index = 1)]
        smsfile: String,
        #[clap(short = 'c', long = "config-dir")]
        config_dir: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SmsEventPayload {
    #[serde(alias = "From")]
    sender: String,
    #[serde(alias = "Sent", deserialize_with = "deserialize_timestamp_from_datetime_string")]
    timestamp: i64,
    text: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let SmsCollectorArgs::Received { smsfile, config_dir } = SmsCollectorArgs::parse();
    let apm_server_api_credentials_filepath =
        format!("{}/{}", config_dir, DEFAULT_APM_SERVER_CREDENTIALS_FILENAME);
    let mut collector_config = build_config(&config_dir)?;

    let apm_credentials_read_result = collector_config
        .logger
        .tracing_elastic_apm
        .read_apm_server_api_credentials_if_not_set(&apm_server_api_credentials_filepath);

    let _guard = setup_logger(collector_config.logger)?;
    if let Err(apm_credentials_read_error) = apm_credentials_read_result {
        warn!("{:?}", apm_credentials_read_error);
    }

    let sms_content = std::fs::read_to_string(&smsfile)?;
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
    let publish_result = match collector_config.sms_collector.tornado_connection_channel {
        TornadoConnectionChannel::Nats { ref nats } => {
            publish_on_nats(serialized_full_event_message, &nats).await
        }
        _ => Err(("Cannot find a valid Tornado connection channel").into()),
    };

    match publish_result {
        Ok(_) => {
            tokio::fs::remove_file(&smsfile).await?;
        }
        Err(err) => {
            error!(
                "The following sms could not be forwarded to Tornado engine: {}. Problem: {}",
                sms_content, err
            );
            let failed_sms_file_name = format!(
                "{}{}",
                collector_config.sms_collector.failed_sms_folder,
                Path::new(&smsfile).file_name().unwrap().to_str().unwrap()
            );
            tokio::fs::rename(&smsfile, &failed_sms_file_name).await?;
        }
    }

    Ok(())
}

async fn publish_on_nats(
    serialized_full_event_message: String,
    nats: &NatsPublisherConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            E: Error,
        {
            match NaiveDateTime::parse_from_str(v, "%y-%m-%d %H:%M:%S") {
                Ok(value) => Ok(value.timestamp()),
                Err(err) => Err(Error::custom(err)),
            }
        }
    }

    deserializer.deserialize_str(DateTimeVisitor)
}
