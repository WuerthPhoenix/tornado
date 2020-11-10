use actix::prelude::*;
use actix_web::web::{Data, Json};
use actix_web::{web, App, HttpServer};
use httpmock::Method::POST;
use httpmock::{Mock, MockServer};
use maplit::*;
use std::sync::Arc;
use std::sync::Mutex;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{StatefulExecutor, ExecutorError};
use tornado_executor_icinga2::config::Icinga2ClientConfig;
use tornado_executor_icinga2::{
    Icinga2Executor, ICINGA2_ACTION_NAME_KEY, ICINGA2_ACTION_PAYLOAD_KEY,
    ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE,
};

#[test]
fn should_perform_a_post_request() {
    println!("start actix System");

    let received = Arc::new(Mutex::new(None));

    let act_received = received.clone();
    System::run(move || {
        let api = "/v1/events";
        let api_clone = api.clone();

        HttpServer::new(move || {
            let app_received = act_received.clone();
            let url = format!("{}{}", api, "/icinga2-api-action");

            App::new().data(app_received).service(web::resource(&url).route(web::post().to(
                move |body: Json<Value>, app_received: Data<Arc<Mutex<Option<Value>>>>| async move {
                    println!("Server received a call");
                    let mut message = app_received.lock().unwrap();
                    *message = Some(body.into_inner());
                    System::current().stop();
                    ""
                },
            )))
        })
        .bind("127.0.0.1:0")
        .and_then(|server| {
            let server_port = server.addrs()[0].port();

            let url = format!("http://127.0.0.1:{}{}", server_port, api_clone);
            println!("Client connecting to: {}", url);

            let config = Icinga2ClientConfig {
                server_api_url: url,
                disable_ssl_verification: true,
                password: "".to_owned(),
                username: "".to_owned(),
                timeout_secs: None,
            };

            std::thread::spawn(move || {
                let mut executor = Icinga2Executor::new(config).unwrap();

                println!("Executor created");

                let mut action = Action::new("");
                action.payload.insert(
                    ICINGA2_ACTION_NAME_KEY.to_owned(),
                    Value::Text("icinga2-api-action".to_owned()),
                );
                action.payload.insert(
                    ICINGA2_ACTION_PAYLOAD_KEY.to_owned(),
                    Value::Map(hashmap![
                        "filter".to_owned() => Value::Text("my_service".to_owned()),
                    ]),
                );

                executor.execute(&action).unwrap();

                println!("Executor action sent");
            });

            Ok(server)
        })
        .expect("Can not bind to port 0")
        .run();
    })
    .unwrap();

    println!("actix System stopped");

    assert_eq!(
        Some(Value::Map(hashmap![
            "filter".to_owned() => Value::Text("my_service".to_owned())
        ])),
        *received.lock().unwrap()
    );
}

#[test]
fn should_return_object_not_existing_error_in_case_of_404_status_code() {
    // Arrange
    let mock_server = MockServer::start();
    let server_response = "{\"error\":404.0,\"status\":\"No objects found.\"}";

    Mock::new()
        .expect_method(POST)
        .expect_path("/icinga2-api-action")
        .return_body(server_response)
        .return_status(404)
        .create_on(&mock_server);

    let mut executor = Icinga2Executor::new(Icinga2ClientConfig {
        timeout_secs: None,
        username: "".to_owned(),
        password: "".to_owned(),
        disable_ssl_verification: true,
        server_api_url: mock_server.url(""),
    })
    .unwrap();

    let mut action = Action::new("");
    action
        .payload
        .insert(ICINGA2_ACTION_NAME_KEY.to_owned(), Value::Text("icinga2-api-action".to_owned()));
    action.payload.insert(
        ICINGA2_ACTION_PAYLOAD_KEY.to_owned(),
        Value::Map(hashmap![
            "filter".to_owned() => Value::Text("my_service".to_owned()),
        ]),
    );

    // Act
    let result = executor.execute(&action);

    // Assert
    assert!(result.is_err());
    assert_eq!(result, Err(ExecutorError::ActionExecutionError { message: format!("Icinga2Executor - Icinga2 API returned an error, object seems to be not existing in Icinga2. Response status: {}. Response body: {}", "404 Not Found", server_response), can_retry: true, code: Some(ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE) }))
}
