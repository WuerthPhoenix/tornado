use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct SmartMonitoringCheckResultConfig {
    /// The number of attempts to perform a process_check_result
    /// for an object in pending state
    pub pending_object_set_status_retries_attempts: u32,

    /// The sleep time in ms between attempts to perform a process_check_result
    /// for an object in pending state
    pub pending_object_set_status_retries_sleep_ms: u64,
}

impl Default for SmartMonitoringCheckResultConfig {
    fn default() -> Self {
        Self {
            pending_object_set_status_retries_attempts: 2,
            pending_object_set_status_retries_sleep_ms: 1000,
        }
    }
}
