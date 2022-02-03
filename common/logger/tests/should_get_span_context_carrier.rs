use opentelemetry::trace::TraceContextExt;
use tornado_common_api::{Event, ValueExt};
use tornado_common_logger::elastic_apm::ApmTracingConfig;
use tornado_common_logger::{setup_logger, LoggerConfig};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[tokio::test]
async fn should_get_span_context_carrier() {
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

    let span_1 = tracing::error_span!("level", "first").entered();
    let mut event = Event::new("some_type");

    // Act
    event.set_trace_context_from_span(&span_1);

    // Assert
    let expected_trace_id = span_1.context().span().span_context().trace_id().to_hex();
    let expected_span_id = span_1.context().span().span_context().span_id().to_hex();

    let metadata = event.metadata.unwrap();
    let trace_context = metadata.get("trace_context").unwrap().get_map().unwrap();
    assert_eq!(trace_context.len(), 2);
    assert!(trace_context.get("traceparent").is_some());
    assert!(trace_context
        .get("traceparent")
        .unwrap()
        .get_text()
        .unwrap()
        .starts_with(&format!("00-{}-{}-", expected_trace_id, expected_span_id)));
    assert_eq!(trace_context.get("tracestate").unwrap().get_text().unwrap(), "");
}
