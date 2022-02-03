use opentelemetry::trace::TraceContextExt;
use serde_json::Value;
use std::collections::HashMap;
use tornado_common_api::{add_metadata_to_span, Event};
use tornado_common_logger::elastic_apm::ApmTracingConfig;
use tornado_common_logger::{setup_logger, LoggerConfig};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[tokio::test]
async fn should_attach_context_to_span() {
    // Arrange
    let config = LoggerConfig {
        stdout_output: false,
        level: "debug".to_owned(),
        file_output_path: None,
        tracing_elastic_apm: ApmTracingConfig {
            apm_output: true,
            apm_server_url: "http://localhost:8200".to_string(),
            apm_server_api_credentials: None,
        },
    };

    let _g = setup_logger(config).unwrap();

    let expected_trace_id = "0af7651916cd43dd8448eb211c80319c";
    let mut trace_context = serde_json::Map::new();
    trace_context.insert(
        "traceparent".to_owned(),
        Value::String(format!("00-{}-b7ad6b7169203331-01", expected_trace_id)),
    );
    trace_context.insert("tracestate".to_owned(), Value::String("".to_owned()));

    let mut event = Event::new("mytype");
    let mut metadata = serde_json::Map::new();
    metadata.insert("trace_context".to_string(), Value::Object(trace_context));
    event.metadata = Some(metadata);
    let span_1 = tracing::debug_span!("level", "first");

    // Act
    add_metadata_to_span(&span_1, &mut event);

    // Assert
    let trace_id = span_1.context().span().span_context().trace_id();
    assert_eq!(expected_trace_id, trace_id.to_hex());
}
