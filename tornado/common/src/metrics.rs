use tornado_common_metrics::opentelemetry::metrics::Counter;
use tornado_common_metrics::opentelemetry::Key;

pub const ACTION_ID_LABEL_KEY: Key = Key::from_static_str("action_id");
pub const ACTION_RESULT_KEY: Key = Key::from_static_str("action_result");
pub const ATTEMPT_RESULT_KEY: Key = Key::from_static_str("attempt_result");
pub const RESULT_SUCCESS: &str = "success";
pub const RESULT_FAILURE: &str = "failure";

pub struct ActionMeter {
    /// Counts the total actions received
    pub actions_received_counter: Counter<u64>,
    /// Counts the total actions processed
    pub actions_processed_counter: Counter<u64>,
    /// Counts the number of the action execution attempts performed
    pub actions_processing_attempts_counter: Counter<u64>,
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

        let actions_processing_attempts_counter = meter
            .u64_counter("actions_processing_attempts_counter")
            .with_description("Counter of the actions execution attempts")
            .init();

        Self {
            actions_received_counter,
            actions_processed_counter,
            actions_processing_attempts_counter,
        }
    }
}
