use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::trace::TraceContextExt;
use serde_json::Value;
use tornado_common_logger::elastic_apm::ApmTracingConfig;
use tornado_common_logger::opentelemetry_logger::TelemetryContextExtractor;
use tornado_common_logger::{setup_logger, LoggerConfig};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[tokio::test]
async fn get_trace_context_should_return_invalid_span_id_if_not_sampled() {
    // Arrange
    let config = LoggerConfig {
        stdout_output: false,
        level: "debug".to_owned(),
        file_output_path: None,
        tracing_elastic_apm: ApmTracingConfig {
            apm_output: true,
            apm_server_url: "http://localhost:8200".to_string(),
            apm_server_api_credentials: None,
            exporter: Default::default(),
        },
    };

    let _g = setup_logger(config).unwrap();

    let expected_trace_id = "0af7651916cd43dd8448eb211c80319c";
    let mut trace_context = serde_json::Map::new();
    trace_context.insert(
        "traceparent".to_owned(),
        Value::String(format!("00-{}-b7ad6b7169203331-00", expected_trace_id)),
    );
    trace_context.insert("tracestate".to_owned(), Value::String("".to_owned()));

    let propagator = TraceContextPropagator::new();

    // Act
    let context = TelemetryContextExtractor::get_trace_context(&trace_context, &propagator);
    let _g = context.clone().attach();
    let span_1 = tracing::debug_span!("level", "first");

    // Assert
    assert_eq!(context.span().span_context().trace_id().to_hex().as_str(), expected_trace_id);
    assert_eq!(context.span().span_context().span_id().to_u64(), 0);
    assert_eq!(
        span_1.context().span().span_context().trace_id().to_hex().as_str(),
        expected_trace_id
    );
    assert_ne!(span_1.context().span().span_context().span_id().to_u64(), 0);
}
