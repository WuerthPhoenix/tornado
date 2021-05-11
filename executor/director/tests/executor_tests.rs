use actix::prelude::*;
use actix_web::web::{Data, Json};
use actix_web::{web, App, HttpServer};
use httpmock::Method::POST;
use httpmock::{Mock, MockServer};
use maplit::*;
use std::sync::Arc;
use std::sync::Mutex;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{StatelessExecutor, ExecutorError};
use tornado_executor_director::config::DirectorClientConfig;
use tornado_executor_director::{
    DirectorExecutor, DIRECTOR_ACTION_NAME_KEY, DIRECTOR_ACTION_PAYLOAD_KEY,
    ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE,
};

#[test]
fn should_perform_a_post_request() {
    println!("start actix System");

    let received = Arc::new(Mutex::new(None));

    let act_received = received.clone();
    System::run(move || {
        let api = "/director";
        let api_clone = api.clone();

        HttpServer::new(move || {
            let app_received = act_received.clone();
            let url = format!("{}{}", api, "/host");

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

            let config = DirectorClientConfig {
                server_api_url: url,
                disable_ssl_verification: true,
                password: "".to_owned(),
                username: "".to_owned(),
                timeout_secs: None,
            };

            actix::spawn(async move {
                let executor = DirectorExecutor::new(config).unwrap();

                println!("Executor created");

                /*
                client_address.do_send(DirectorApiClientMessage {
                    message: DirectorAction {
                        name: DirectorActionName::CreateHost,
                        payload: Value::Map(hashmap![
                            "object_type".to_owned() => Value::Text("host".to_owned()),
                            "object_name".to_owned() => Value::Text("my_host".to_owned()),
                            "address".to_owned() => Value::Text("127.0.0.1".to_owned()),
                            "check_command".to_owned() => Value::Text("hostalive".to_owned())
                        ]),
                        live_creation: false
                    },
                });
                    */

                let mut action = Action::new("");
                action.payload.insert(
                    DIRECTOR_ACTION_NAME_KEY.to_owned(),
                    Value::Text("create_host".to_owned()),
                );
                action.payload.insert(
                        DIRECTOR_ACTION_PAYLOAD_KEY.to_owned(),
                        Value::Map(hashmap![
                            "object_type".to_owned() => Value::Text("host".to_owned()),
                            "object_name".to_owned() => Value::Text("my_host".to_owned()),
                            "address".to_owned() => Value::Text("127.0.0.1".to_owned()),
                            "check_command".to_owned() => Value::Text("hostalive".to_owned())
            ]),
                    );

                executor.execute(action.into()).await.unwrap();

                println!("DirectorApiClientMessage action sent");
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
            "object_type".to_owned() => Value::Text("host".to_owned()),
            "object_name".to_owned() => Value::Text("my_host".to_owned()),
            "address".to_owned() => Value::Text("127.0.0.1".to_owned()),
            "check_command".to_owned() => Value::Text("hostalive".to_owned())
        ])),
        *received.lock().unwrap()
    );
}

#[actix_rt::test]
async fn should_return_object_already_existing_error_in_case_of_422_status_code() {
    // Arrange
    let director_server = MockServer::start();
    let server_response = "{\"error\": \"Trying to recreate icinga_host (\"some host\")\"}";

    Mock::new()
        .expect_method(POST)
        .expect_path("/host")
        .return_body(server_response)
        .return_status(422)
        .create_on(&director_server);

    let executor = DirectorExecutor::new(DirectorClientConfig {
        timeout_secs: None,
        username: "".to_owned(),
        password: "".to_owned(),
        disable_ssl_verification: true,
        server_api_url: director_server.url(""),
    })
    .unwrap();

    let mut action = Action::new("");
    action
        .payload
        .insert(DIRECTOR_ACTION_NAME_KEY.to_owned(), Value::Text("create_host".to_owned()));
    action.payload.insert(
        DIRECTOR_ACTION_PAYLOAD_KEY.to_owned(),
        Value::Map(hashmap![
                        "object_type".to_owned() => Value::Text("host".to_owned()),
                        "object_name".to_owned() => Value::Text("my_host".to_owned()),
                        "address".to_owned() => Value::Text("127.0.0.1".to_owned()),
                        "check_command".to_owned() => Value::Text("hostalive".to_owned())
        ]),
    );

    // Act
    let result = executor.execute(action.into()).await;

    // Assert
    assert!(result.is_err());
    assert_eq!(result, Err(ExecutorError::ActionExecutionError { message: format!("DirectorExecutor - Icinga Director API returned an error, object seems to be already existing. Response status: {}. Response body: {}", "422 Unprocessable Entity", server_response), can_retry: true, code: Some(ICINGA2_OBJECT_ALREADY_EXISTING_EXECUTOR_ERROR_CODE) }))
}
