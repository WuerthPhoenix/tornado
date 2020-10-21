use tornado_common_api::{Payload, Value};
use tornado_executor_icinga2::Icinga2Action;
use tornado_executor_director::{DirectorAction, DirectorActionName};
use serde::{Deserialize, Serialize};

const PROCESS_CHECK_RESULT_SUBURL: &str = "process-check-result";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "action_name")]
pub enum MonitoringAction {
    #[serde(rename = "create_and_or_process_host_passive_check_result")]
    Host { process_check_result_payload: Payload, host_creation_payload: Value },
    #[serde(rename = "create_and_or_process_service_passive_check_result")]
    Service {
        process_check_result_payload: Payload,
        host_creation_payload: Value,
        service_creation_payload: Value,
    },
    #[serde(rename = "simple_create_and_or_process_passive_check_result")]
    SimpleCreateAndProcess { check_result: Payload, host: Payload, service: Option<Payload> },
}

impl MonitoringAction {
    // Transforms the MonitoringAction into the actions needed to call the IcingaExecutor and the
    // DirectorExecutor.
    // Returns a triple, with these elements:
    // 1. Icinga2Action that will perform the process-check-result through the IcingaExecutor
    // 2. DirectorAction that will perform the creation of the host through the DirectorAction
    // 3. Option<DirectorAction> that will perform the creation of the service through the
    // DirectorAction. This is Some if MonitoringAction is of type Service, None otherwise
    pub fn to_sub_actions(&self) -> (Icinga2Action, DirectorAction, Option<DirectorAction>) {
        match &self {
            MonitoringAction::Host { process_check_result_payload, host_creation_payload } => (
                Icinga2Action {
                    name: PROCESS_CHECK_RESULT_SUBURL,
                    payload: Some(process_check_result_payload),
                },
                DirectorAction {
                    name: DirectorActionName::CreateHost,
                    payload: host_creation_payload,
                    live_creation: true,
                },
                None,
            ),
            MonitoringAction::Service {
                process_check_result_payload,
                host_creation_payload,
                service_creation_payload,
            } => (
                Icinga2Action {
                    name: PROCESS_CHECK_RESULT_SUBURL,
                    payload: Some(process_check_result_payload),
                },
                DirectorAction {
                    name: DirectorActionName::CreateHost,
                    payload: host_creation_payload,
                    live_creation: true,
                },
                Some(DirectorAction {
                    name: DirectorActionName::CreateService,
                    payload: service_creation_payload,
                    live_creation: true,
                }),
            ),
            MonitoringAction::SimpleCreateAndProcess { check_result, host, service } => {
                let remove_me = 0;
                unimplemented!()
            }
        }
    }
}