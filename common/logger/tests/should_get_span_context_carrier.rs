use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::trace::TraceContextExt;
use tornado_common_api::ValueExt;
use tornado_common_logger::elastic_apm::ApmTracingConfig;
use tornado_common_logger::opentelemetry_logger::TelemetryContextInjector;
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
            exporter: Default::default(),
        },
    };

    let _g = setup_logger(config).unwrap();

    let span_1 = tracing::error_span!("level", "first").entered();
    let propagator = TraceContextPropagator::new();

    // Act
    let res = TelemetryContextInjector::get_trace_context_map(&span_1.context(), &propagator);

    // Assert
    let expected_trace_id = span_1.context().span().span_context().trace_id().to_hex();
    let expected_span_id = span_1.context().span().span_context().span_id().to_hex();

    assert_eq!(res.len(), 2);
    assert!(res.get("traceparent").is_some());
    assert!(res
        .get("traceparent")
        .unwrap()
        .get_text()
        .unwrap()
        .starts_with(&format!("00-{}-{}-", expected_trace_id, expected_span_id)));
    assert_eq!(res.get("tracestate").unwrap().get_text().unwrap(), "");
}
