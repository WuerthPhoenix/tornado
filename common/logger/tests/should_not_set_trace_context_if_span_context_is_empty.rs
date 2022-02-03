use tornado_common_api::Event;

#[tokio::test]
async fn should_not_set_trace_context_if_empty_span_context() {
    // Arrange
    let span_1 = tracing::error_span!("level", "first").entered();
    let mut event = Event::new("some_type");

    // Act
    event.set_trace_context_from_span(&span_1);

    // Assert
    assert!(event.metadata.is_none());
}
