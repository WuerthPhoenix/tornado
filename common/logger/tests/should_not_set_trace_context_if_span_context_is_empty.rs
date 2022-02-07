use opentelemetry::sdk::propagation::TraceContextPropagator;
use tornado_common_logger::opentelemetry_logger::TelemetryContextInjector;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[tokio::test]
async fn get_trace_context_map_should_return_empty_map_for_empty_context() {
    // Arrange
    let span_1 = tracing::error_span!("level", "first").entered();
    let trace_context_propagator = TraceContextPropagator::new();

    // Act
    let res = TelemetryContextInjector::get_trace_context_map(
        &span_1.context(),
        &trace_context_propagator,
    );

    // Assert
    assert!(res.is_empty());
}
