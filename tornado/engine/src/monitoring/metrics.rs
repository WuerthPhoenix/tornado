use tornado_common_metrics::opentelemetry::metrics::{Counter, ValueRecorder, Unit};

pub const TORNADO_APP: &str = "tornado";

pub struct TornadoMeter {
    pub http_requests_counter: Counter<u64>,
    pub http_requests_duration_seconds: ValueRecorder<f64>,
}

impl Default for TornadoMeter {

    fn default() -> Self {
        let meter = tornado_common_metrics::opentelemetry::global::meter("tornado");

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
            http_requests_counter,
            http_requests_duration_seconds
        }
    }

}