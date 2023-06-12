use crate::action::SimpleCreateAndProcess;
use tornado_common_api::{Action, Payload, Value};
use tornado_executor_common::ExecutorError;
use tornado_executor_monitoring::MonitoringAction;

const PAYLOAD_CHECK_RESULT_KEY: &str = "check_result";
const PAYLOAD_HOST_KEY: &str = "host";
const PAYLOAD_SERVICE_KEY: &str = "service";

/// This function accepts a valid monitoring action Payload and converts it into a
/// valid SimpleCreateAndProcess action payload
pub fn migrate_from_monitoring(input: &Payload) -> Result<Payload, ExecutorError> {
    let monitoring_action =
        tornado_executor_monitoring::MonitoringExecutor::parse_monitoring_action(input)?;

    let mut output = Payload::new();

    match monitoring_action {
        MonitoringAction::Host { mut process_check_result_payload, mut host_creation_payload } => {
            remove_entries(&mut process_check_result_payload);
            output.insert(
                PAYLOAD_CHECK_RESULT_KEY.to_owned(),
                Value::Object(process_check_result_payload),
            );

            remove_entries(&mut host_creation_payload);
            output.insert(PAYLOAD_HOST_KEY.to_owned(), Value::Object(host_creation_payload));
        }
        MonitoringAction::Service {
            mut process_check_result_payload,
            mut host_creation_payload,
            mut service_creation_payload,
        } => {
            remove_entries(&mut process_check_result_payload);
            output.insert(
                PAYLOAD_CHECK_RESULT_KEY.to_owned(),
                Value::Object(process_check_result_payload),
            );

            remove_entries(&mut host_creation_payload);
            output.insert(PAYLOAD_HOST_KEY.to_owned(), Value::Object(host_creation_payload));

            remove_entries(&mut service_creation_payload);
            output.insert(PAYLOAD_SERVICE_KEY.to_owned(), Value::Object(service_creation_payload));
        }
    }

    // Verify the generated payload is valid
    SimpleCreateAndProcess::new(&Action::new_with_payload_and_created_ms("", output.clone(), 0))?;

    Ok(output)
}

fn remove_entries(payload: &mut Payload) {
    payload.remove("host");
    payload.remove("type");
    payload.remove("service");
    payload.remove("object_type");
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::action::test::compare_actions_discard_execution_start_execution_end;
    use tornado_engine_matcher::config::rule::ConfigAction;

    #[test]
    fn test_before_and_after_migration() {
        check_migration(
            "./tests_resources/migration/monitoring_host_01_source.json",
            "./tests_resources/migration/monitoring_host_01_dest.json",
        );

        check_migration(
            "./tests_resources/migration/monitoring_service_01_source.json",
            "./tests_resources/migration/monitoring_service_01_dest.json",
        );

        check_migration(
            "./tests_resources/migration/monitoring_service_02_source.json",
            "./tests_resources/migration/monitoring_service_02_dest.json",
        );
    }

    fn check_migration(source_action_filename: &str, dest_action_filename: &str) {
        println!("Check migration from {} to {}", source_action_filename, dest_action_filename);

        // Arrange
        let source_action = to_action(source_action_filename);
        let dest_action = to_action(dest_action_filename);

        // Act
        let migrated_payload = migrate_from_monitoring(&source_action.payload).unwrap();
        let migrated_action = ConfigAction {
            id: "smart_monitoring_check_result".to_string(),
            payload: migrated_payload.clone(),
        };

        // Assert
        assert_eq!(dest_action, migrated_action);

        let monitoring_action =
            tornado_executor_monitoring::MonitoringExecutor::parse_monitoring_action(
                &source_action.payload,
            )
            .unwrap();
        let smart_monitoring_action = Action {
            id: "smart_monitoring_check_result".to_string(),
            payload: migrated_payload,
            created_ms: 1650643471000,
        };
        let mut smart_monitoring_action =
            SimpleCreateAndProcess::new(&smart_monitoring_action).unwrap();
        let monitoring_sub_actions = monitoring_action.to_sub_actions().unwrap();
        let smart_monitoring_sub_actions = smart_monitoring_action.build_sub_actions().unwrap();
        compare_actions_discard_execution_start_execution_end(
            monitoring_sub_actions,
            smart_monitoring_sub_actions,
        );
    }

    fn to_action(filename: &str) -> ConfigAction {
        let json = std::fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename));
        serde_json::from_str(&json).unwrap()
    }
}
