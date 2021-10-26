use tornado_common_metrics::opentelemetry::metrics::{Counter};
use tornado_common_metrics::opentelemetry::Key;

pub const TORNADO_APP: &str = "tornado";
pub const ACTION_ID_LABEL_KEY: Key = Key::from_static_str("action_id");
pub const ACTION_RESULT_KEY: Key = Key::from_static_str("action_result");
pub const ACTION_RESULT_SUCCESS: &str = "success";
pub const ACTION_RESULT_FAILURE: &str = "failure";


pub struct ActionMeter {
    /// Counts the total actions received
    pub actions_received_counter: Counter<u64>,
    /// Counts the total actions processed
    pub actions_processed_counter: Counter<u64>,
    /// Counts the total action retries performed
    pub action_retries_counter: Counter<u64>,
}

impl ActionMeter {
    pub fn new(meter_name: &'static str) -> Self {
        let meter = tornado_common_metrics::opentelemetry::global::meter(meter_name);

        let actions_received_counter = meter
            .u64_counter("actions_received_counter")
            .with_description("Actions received count")
            .init();

        let actions_processed_counter = meter
            .u64_counter("actions_processed_counter")
            .with_description("Actions processed count")
            .init();

        let action_retries_counter = meter
            .u64_counter("action_retries_counter")
            .with_description("Action retries performed count")
            .init();

        Self {
            actions_received_counter,
            actions_processed_counter,
            action_retries_counter
        }

    }
}
