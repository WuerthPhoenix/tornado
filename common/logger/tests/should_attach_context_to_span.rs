use opentelemetry::trace::TraceContextExt;
use std::collections::HashMap;
use tornado_common_logger::elastic_apm::ApmTracingConfig;
use tornado_common_logger::opentelemetry_logger::attach_context_to_span;
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
    let mut carrier = HashMap::new();
    carrier
        .insert("traceparent".to_owned(), format!("00-{}-b7ad6b7169203331-01", expected_trace_id));
    carrier.insert("tracestate".to_owned(), "".to_owned());
    let span_1 = tracing::debug_span!("level", "first");

    // Act
    attach_context_to_span(&span_1, Some(carrier));

    // Assert
    let trace_id = span_1.context().span().span_context().trace_id();
    assert_eq!(expected_trace_id, trace_id.to_hex());
}
