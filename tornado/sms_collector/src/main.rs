mod config;
mod error;
mod sms;

use crate::config::{build_config, CollectorConfig};
use crate::error::{SmsCollectorConfigError, SmsCollectorError};
use crate::sms::parse_sms;
use clap::Parser;
use log::*;
use opentelemetry::trace::SpanKind;
use std::io;
use std::path::PathBuf;
use std::process::exit;
use tornado_common::actors::nats_publisher::NatsPublisherConfig;
use tornado_common_api::{Event, Value};
use tornado_common_logger::elastic_apm::DEFAULT_APM_SERVER_CREDENTIALS_FILENAME;
use tornado_common_logger::opentelemetry_logger::TelemetryContextInjector;
use tornado_common_logger::{setup_logger, LogWorkerGuard};
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
    let (args, collector_config, _logger_guard) = match load_config() {
        Ok(res) => res,
        Err(error) => {
            println!("Could not load config: {error}");
            exit(1);
        }
    };

    if args.event != SMS_EVENT_HANDLE_ACTION {
        info!("Ignoring event \"{}\"", args.event);
        return Ok(());
    }

    let collector_result = execute_collector(&args.sms_file, collector_config).await;

    match &collector_result {
        Ok(()) => tokio::fs::remove_file(&args.sms_file).await,
        Err(err @ SmsCollectorError::TornadoConnectionError { failed_sms_file, .. }) => {
            error!("Could not forward the sms to Tornado. Error: {err}");
            std::fs::rename(&args.sms_file, failed_sms_file)
        }
        Err(err) => {
            error!("Could not process sms file: {err}");
            exit(1);
        }
    }
}

fn load_config(
) -> Result<(SmsCollectorArgs, CollectorConfig, LogWorkerGuard), SmsCollectorConfigError> {
    let args = SmsCollectorArgs::try_parse()?;

    let mut collector_config = build_config(&args.config_dir)?;

    let logger_guard = setup_logger(collector_config.logger.clone())?;

    let apm_server_api_credentials_filepath =
        format!("{}/{}", args.config_dir, DEFAULT_APM_SERVER_CREDENTIALS_FILENAME);
    let apm_credentials_read_result = collector_config
        .logger
        .tracing_elastic_apm
        .read_apm_server_api_credentials_if_not_set(&apm_server_api_credentials_filepath);

    if let Err(apm_credentials_read_error) = apm_credentials_read_result {
        warn!("{:?}", apm_credentials_read_error);
    }

    Ok((args, collector_config, logger_guard))
}

async fn execute_collector(
    sms_file: &SmsFile,
    collector_config: CollectorConfig,
) -> Result<(), SmsCollectorError> {
    let sms_content = match std::fs::read_to_string(sms_file) {
        Ok(sms_content) => sms_content,
        Err(err) => {
            return Err(SmsCollectorError::SmsFileAccessError {
                error: err,
                sms_file: sms_file.clone(),
            });
        }
    };
    let mut full_event_message = match parse_sms(&sms_content) {
        Ok(sms_event_payload) => {
            let Ok(Value::Object(payload)) = serde_json::to_value(sms_event_payload) else {
                // The transform from a struct to the payload will always succeed.
                unreachable!()
            };
            Event::new_with_payload("sms", payload)
        }
        Err(parse_error) => return Err(SmsCollectorError::SmsParseError { error: parse_error }),
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
    let nats = collector_config.sms_collector.tornado_connection_channel.nats;
    if let Err(err) = publish_on_nats(serialized_full_event_message, &nats).await {
        let mut failed_sms_file = collector_config.sms_collector.failed_sms_folder;
        let sms_file = sms_file.file_name().expect("filename of already read file to be present.");
        failed_sms_file.push(sms_file);

        return Err(SmsCollectorError::TornadoConnectionError { error: err, failed_sms_file });
    }

    Ok(())
}

async fn publish_on_nats(
    serialized_full_event_message: String,
    nats: &NatsPublisherConfig,
) -> io::Result<()> {
    info!("Connect to Tornado through NATS");
    let nats_connection = nats.client.new_client().await?;
    nats_connection.publish(&nats.subject, serialized_full_event_message).await?;
    nats_connection.flush().await?;
    nats_connection.close().await
}
