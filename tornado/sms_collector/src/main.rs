mod config;
mod error;
mod sms;

use crate::config::build_config;
use crate::error::SmsCollectorError;
use crate::sms::parse_sms;
use clap::Parser;
use log::*;
use opentelemetry::trace::SpanKind;
use std::io;
use std::path::PathBuf;
use tornado_common::actors::nats_publisher::NatsPublisherConfig;
use tornado_common_api::{Event, Value};
use tornado_common_logger::elastic_apm::DEFAULT_APM_SERVER_CREDENTIALS_FILENAME;
use tornado_common_logger::opentelemetry_logger::TelemetryContextInjector;
use tornado_common_logger::setup_logger;
use tornado_common_metrics::opentelemetry::sdk::propagation::TraceContextPropagator;
use tracing_opentelemetry::OpenTelemetrySpanExt;

const SMS_EVENT_HANDLE_ACTION: &str = "RECEIVED";

type SmsFile = PathBuf;

#[derive(Parser)]
struct SmsCollectorArgs {
    #[clap(short = 'c', long = "config-dir")]
    config_dir: String,
    #[clap(index = 1)]
    event: String,
    #[clap(index = 2)]
    sms_file: SmsFile,
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
    let mut full_event_message = match parse_sms(&sms_content) {
        Ok(sms_event_payload) => {
            let Ok(Value::Object(payload)) = serde_json::to_value(sms_event_payload) else {
                // The transform form a struct to the payload will always succeed.
                unreachable!()
            };
            Event::new_with_payload("sms", payload)
        }
        Err(parse_error) => {
            return Err(SmsCollectorError::SmsParseError {
                error: parse_error,
                sms_file: args.sms_file,
            })
        }
    };

    let span = tracing::info_span!("Received SMS", trace_id = tracing::field::Empty, 
            otel.kind = %SpanKind::Producer,
            peer.service = "tornado")
    .entered();

    // Instantiate tracing for the event
    let trace_id = full_event_message.get_trace_id_for_logging(&span.context());
    span.record("trace_id", &trace_id.as_ref());
    let trace_context_propagator = TraceContextPropagator::new();
    let trace_context =
        TelemetryContextInjector::get_trace_context_map(&span.context(), &trace_context_propagator);
    full_event_message.set_trace_context(trace_context);

    // Send message to nats
    let serialized_full_event_message =
        serde_json::to_string(&full_event_message).expect("payload is always serializable");
    let nats = collector_config.sms_collector.tornado_connection_channel;
    match publish_on_nats(serialized_full_event_message, &nats).await {
        Ok(_) => Ok(args.sms_file),
        Err(err) => {
            let mut failed_sms_file =
                PathBuf::from(collector_config.sms_collector.failed_sms_folder);
            let sms_file =
                &args.sms_file.file_name().expect("filename of already read file to be present.");
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
