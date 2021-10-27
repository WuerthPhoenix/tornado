use opentelemetry::sdk::Resource;
use opentelemetry::KeyValue;
use opentelemetry_prometheus::PrometheusExporter;

pub use opentelemetry;
pub use prometheus;

pub mod endpoint;

pub struct Metrics {
    pub prometheus_exporter: PrometheusExporter,
}

impl Metrics {
    pub fn new(app_name: &'static str) -> Self {
        let prometheus_exporter = opentelemetry_prometheus::exporter()
            .with_resource(Resource::new(vec![KeyValue::new("app", app_name)]))
            .init();
        Self { prometheus_exporter }
    }
}
