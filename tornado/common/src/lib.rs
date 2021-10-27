use thiserror::Error;

pub mod actors;
pub mod command;
pub mod metrics;

#[derive(Error, Debug)]
pub enum TornadoError {
    #[error("SenderError: {message}")]
    SenderError { message: String },
    #[error("ActorCreationError: {message}")]
    ActorCreationError { message: String },
    #[error("ConfigurationError: {message}")]
    ConfigurationError { message: String },
    #[error("ExecutionError: {message}")]
    ExecutionError { message: String },
}

#[cfg(test)]
pub mod root_test {
    use once_cell::sync::OnceCell;
    use opentelemetry_prometheus::PrometheusExporter;
    use tornado_common_metrics::opentelemetry::sdk::Resource;
    use tornado_common_metrics::opentelemetry::KeyValue;

    pub fn prometheus_exporter() -> &'static PrometheusExporter {
        static PROMETHEUS_EXPORTER: OnceCell<PrometheusExporter> = OnceCell::new();
        PROMETHEUS_EXPORTER.get_or_init(|| {
            opentelemetry_prometheus::exporter()
                .with_resource(Resource::new(vec![KeyValue::new("app", "test_app")]))
                .init()
        })
    }
}
