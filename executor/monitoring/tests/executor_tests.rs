use httpmock::Method::POST;
use httpmock::{Mock, MockServer, Regex};
use maplit::*;
use std::collections::HashMap;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{Executor, ExecutorError};
use tornado_executor_director::config::DirectorClientConfig;
use tornado_executor_icinga2::config::Icinga2ClientConfig;
use tornado_executor_monitoring::MonitoringExecutor;

#[test]
fn should_return_error_if_process_check_result_fails_with_error_different_than_non_existing_object()
{
    // Arrange
    let icinga_server = MockServer::start();

    let icinga_mock = Mock::new()
        .expect_method(POST)
        .expect_path("/v1/actions/process-check-result")
        .return_status(500)
        .create_on(&icinga_server);

    let mut executor = MonitoringExecutor::new(
        Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: icinga_server.url(""),
        },
        DirectorClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: "".to_owned(),
        },
    )
    .unwrap();

    let mut action = Action::new("");
    action.payload.insert(
        "action_name".to_owned(),
        Value::Text("create_and_or_process_host_passive_check_result".to_owned()),
    );
    action.payload.insert(
        "process_check_result_payload".to_owned(),
        Value::Map(hashmap!(
            "host".to_owned() => Value::Text("myhost".to_owned()),
        )),
    );
    action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));

    // Act
    let result = executor.execute(&action);

    // Assert
    assert!(result.is_err());
    assert_eq!(icinga_mock.times_called(), 1);
    assert_eq!(result, Err(ExecutorError::ActionExecutionError { message: format!("MonitoringExecutor - Error while performing the process check result. IcingaExecutor failed with error: ActionExecutionError {{ message: \"Icinga2Executor - Icinga2 API returned an error. Response status: {}. Response body: {}\", can_retry: true, code: None }}", "500 Internal Server Error", ""), can_retry: true, code: None }))
}

#[test]
fn should_return_ok_if_process_check_result_is_successful() {
    // Arrange
    let icinga_server = MockServer::start();

    let icinga_mock = Mock::new()
        .expect_method(POST)
        .expect_path("/v1/actions/process-check-result")
        .return_status(201)
        .create_on(&icinga_server);

    let mut executor = MonitoringExecutor::new(
        Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: icinga_server.url(""),
        },
        DirectorClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: "".to_owned(),
        },
    )
    .unwrap();

    let mut action = Action::new("");
    action.payload.insert(
        "action_name".to_owned(),
        Value::Text("create_and_or_process_host_passive_check_result".to_owned()),
    );
    action.payload.insert(
        "process_check_result_payload".to_owned(),
        Value::Map(hashmap!(
            "host".to_owned() => Value::Text("myhost".to_owned()),
        )),
    );
    action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));

    // Act
    let result = executor.execute(&action);

    // Assert
    assert!(result.is_ok());
    assert_eq!(icinga_mock.times_called(), 1);
}

#[test]
fn should_return_call_process_check_result_twice_on_non_existing_object() {
    // Arrange
    let icinga_server = MockServer::start();
    let icinga_server_response = "{\"error\":404.0,\"status\":\"No objects found.\"}";

    let icinga_mock = Mock::new()
        .expect_method(POST)
        .expect_path("/v1/actions/process-check-result")
        .return_body(icinga_server_response)
        .return_status(404)
        .create_on(&icinga_server);

    let director_server = MockServer::start();

    let director_mock = Mock::new()
        .expect_method(POST)
        .expect_path_matches(Regex::new("/(host)|(service)").unwrap())
        .return_status(201)
        .create_on(&director_server);

    let mut executor = MonitoringExecutor::new(
        Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: icinga_server.url(""),
        },
        DirectorClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: director_server.url(""),
        },
    )
    .unwrap();

    let mut action = Action::new("");
    action.payload.insert(
        "action_name".to_owned(),
        Value::Text("create_and_or_process_service_passive_check_result".to_owned()),
    );
    action.payload.insert(
        "process_check_result_payload".to_owned(),
        Value::Map(hashmap!(
            "service".to_owned() => Value::Text("myhost:myservice".to_owned()),
        )),
    );
    action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));
    action.payload.insert("service_creation_payload".to_owned(), Value::Map(HashMap::new()));

    // Act
    let result = executor.execute(&action);

    // Assert
    assert!(result.is_err());
    // one time when object is not existing, one time after the creation of the object
    assert_eq!(icinga_mock.times_called(), 2);
    // director server should be called once to create the host, and once to create the service
    assert_eq!(director_mock.times_called(), 2);
    assert_eq!(result, Err(ExecutorError::ActionExecutionError { message: format!("MonitoringExecutor - Error while performing the process check result after the object creation. IcingaExecutor failed with error: ActionExecutionError {{ message: \"Icinga2Executor - Icinga2 API returned an error, object seems to be not existing in Icinga2. Response status: {}. Response body: {}\", can_retry: true, code: Some(\"IcingaObjectNotExisting\") }}", "404 Not Found", icinga_server_response.escape_debug()), can_retry: true, code: None  }))
}

#[test]
fn should_return_return_error_on_object_creation_failure() {
    // Arrange
    let icinga_server = MockServer::start();
    let icinga_server_response = "{\"error\":404.0,\"status\":\"No objects found.\"}";

    let icinga_mock = Mock::new()
        .expect_method(POST)
        .expect_path("/v1/actions/process-check-result")
        .return_body(icinga_server_response)
        .return_status(404)
        .create_on(&icinga_server);

    let director_server = MockServer::start();
    let director_server_response = "{\"error\":500.0,\"status\":\"Internal Server Error.\"}";

    let director_mock = Mock::new()
        .expect_method(POST)
        .expect_path_matches(Regex::new("/(host)|(service)").unwrap())
        .return_body(director_server_response)
        .return_status(500)
        .create_on(&director_server);

    let mut executor = MonitoringExecutor::new(
        Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: icinga_server.url(""),
        },
        DirectorClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: director_server.url(""),
        },
    )
    .unwrap();

    let mut action = Action::new("");
    action.payload.insert(
        "action_name".to_owned(),
        Value::Text("create_and_or_process_service_passive_check_result".to_owned()),
    );
    action.payload.insert(
        "process_check_result_payload".to_owned(),
        Value::Map(hashmap!(
            "service".to_owned() => Value::Text("myhost:myservice".to_owned()),
        )),
    );
    action.payload.insert("host_creation_payload".to_owned(), Value::Map(HashMap::new()));
    action.payload.insert("service_creation_payload".to_owned(), Value::Map(HashMap::new()));

    // Act
    let result = executor.execute(&action);

    // Assert
    assert!(result.is_err());
    // one time when object is not existing, one time after the creation of the object
    assert_eq!(icinga_mock.times_called(), 1);
    // director server should be called once to create the host, and once to create the service
    assert_eq!(director_mock.times_called(), 1);
    assert_eq!(
        result,
        Err(ExecutorError::ActionExecutionError {
            message: format!(
                "MonitoringExecutor - Error during the host creation. DirectorExecutor failed with error: ActionExecutionError {{ message: \"DirectorExecutor API returned an error. Response status: {}. Response body: {}\", can_retry: true, code: None }}",
                "500 Internal Server Error", director_server_response.escape_debug()
            ),
            can_retry: true,
            code: None
        })
    )
}
