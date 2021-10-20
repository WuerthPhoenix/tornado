use opentelemetry::sdk::Resource;
use opentelemetry::KeyValue;
use opentelemetry_prometheus::PrometheusExporter;
use opentelemetry::metrics::{Counter, ValueRecorder, Unit};

pub use opentelemetry;
pub use prometheus;

pub mod endpoint;

pub struct Metrics {
    pub prometheus_exporter: PrometheusExporter,
    pub http_requests_counter: Counter<u64>,
    pub http_requests_duration_seconds: ValueRecorder<f64>,
}

impl Default for Metrics {

    fn default() -> Self {
        let prometheus_exporter = opentelemetry_prometheus::exporter()
            .with_resource(Resource::new(vec![KeyValue::new("app", "tornado")]))
            .init();

        let meter = opentelemetry::global::meter("tornado");

        let http_requests_counter = meter
            .u64_counter("http_requests.counter")
            .with_description("Counts requests")
            .init();

        let http_requests_duration_seconds = meter
            .f64_value_recorder("http_requests.duration_secs")
            .with_description("HTTP request duration per route")
            .with_unit(Unit::new("seconds"))
            .init();

        Self {
            prometheus_exporter,
            http_requests_counter,
            http_requests_duration_seconds
        }
    }

}