use serde::{Deserialize, Serialize};

use super::CheckResult;

/// The `event:CheckResult` message has the following parts in the parameters:
///
///     * It always contains a host on which the check was performed
///     * If the check was performed for a service, the service is present as well.
///     * The CheckResult as described in [here](icinga2::message::check_result::CheckResult)
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct CheckResultParams {
    pub host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(default)]
    pub cr: Box<CheckResult>,
}

impl CheckResultParams {
    pub fn for_host(host: String, cr: Box<CheckResult>) -> CheckResultParams {
        CheckResultParams {
            host,
            service: None,
            cr,
        }
    }

    pub fn for_service(host: String, service: String, cr: Box<CheckResult>) -> CheckResultParams {
        CheckResultParams {
            host,
            service: Some(service),
            cr,
        }
    }
}
