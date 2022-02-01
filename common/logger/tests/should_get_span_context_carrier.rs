use tornado_common_logger::elastic_apm::ApmTracingConfig;
use tornado_common_logger::opentelemetry_logger::get_span_context_carrier;
use tornado_common_logger::{setup_logger, LoggerConfig};

#[tokio::test]
async fn should_get_span_context_carrier() {
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
    let context_carrier = get_span_context_carrier(&span_1);

    assert_eq!(context_carrier.len(), 2);
    assert!(context_carrier.get("traceparent").is_some());
    assert!(context_carrier.get("traceparent").unwrap().starts_with("00-"));
    assert_eq!(context_carrier.get("tracestate"), Some(&"".to_owned()));
}
