use actix_web::web::{Data, Json};
use actix_web::{web, App, HttpServer};
use httpmock::Method::POST;
use httpmock::MockServer;
use maplit::*;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{ExecutorError, StatelessExecutor};
use tornado_executor_icinga2::config::Icinga2ClientConfig;
use tornado_executor_icinga2::{
    Icinga2Executor, ICINGA2_ACTION_NAME_KEY, ICINGA2_ACTION_PAYLOAD_KEY,
    ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE,
};

#[actix_rt::test]
async fn should_perform_a_post_request() {
    println!("start actix System");

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    actix_rt::spawn(async move {
        let api = "/v1/events";
        let api_clone = api.clone();

        HttpServer::new(move || {
            let url = format!("{}{}", api, "/v1/actions/icinga2-api-action");
            let sender = sender.clone();
            App::new().app_data(Data::new(Arc::new(sender))).service(web::resource(&url).route(web::post().to(
                move |body: Json<Value>, sender: Data<Arc<UnboundedSender<Value>>>| async move {
                    println!("Server received a call");
                    sender.send(body.into_inner()).unwrap();
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

            actix_rt::spawn(async move {
                let executor = Icinga2Executor::new(config).unwrap();

                println!("Executor created");

                let mut action = Action::new("","");
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

                executor.execute(action.into()).await.unwrap();

                println!("Executor action sent");
            });

            Ok(server)
        })
        .expect("Can not bind to port 0")
        .run()
        .await
        .unwrap();
    });

    assert_eq!(
        Some(Value::Map(hashmap![
            "filter".to_owned() => Value::Text("my_service".to_owned())
        ])),
        receiver.recv().await
    );
}

#[tokio::test]
async fn should_return_object_not_existing_error_in_case_of_404_status_code() {
    // Arrange
    let server = MockServer::start();
    let server_response = "{\"error\":404.0,\"status\":\"No objects found.\"}";

    server.mock(|when, then| {
        when.method(POST).path("/v1/actions/icinga2-api-action");
        then.body(server_response).status(404);
    });

    let executor = Icinga2Executor::new(Icinga2ClientConfig {
        timeout_secs: None,
        username: "".to_owned(),
        password: "".to_owned(),
        disable_ssl_verification: true,
        server_api_url: server.url(""),
    })
    .unwrap();

    let mut action = Action::new("","");
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
    let result = executor.execute(action.clone().into()).await;

    // Assert
    assert!(result.is_err());
    assert_eq!(result, Err(ExecutorError::ActionExecutionError {
        message: format!("Icinga2Executor - Icinga2 API returned an error, object seems to be not existing in Icinga2. Response status: {}. Response body: {}", "404 Not Found", server_response),
        can_retry: true,
        code: Some(ICINGA2_OBJECT_NOT_EXISTING_EXECUTOR_ERROR_CODE),
        data: hashmap! {
            "method" => "POST".into(),
            "url" => format!("{}/v1/actions/icinga2-api-action", server.url("")).into(),
            "payload" => serde_json::to_value(action.payload.get(ICINGA2_ACTION_PAYLOAD_KEY)).unwrap()
        }.into(),
    }))
}
