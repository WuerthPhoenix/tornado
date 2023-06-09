use actix_web::web::{Data, Json};
use actix_web::{web, App, HttpServer};
use httpmock::Method::POST;
use httpmock::MockServer;
use maplit::*;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tornado_common_api::{Action, Value};
use tornado_executor_common::StatelessExecutor;
use tornado_executor_director::config::DirectorClientConfig;
use tornado_executor_director::{
    DirectorExecutor, DIRECTOR_ACTION_NAME_KEY, DIRECTOR_ACTION_PAYLOAD_KEY,
};

#[actix_rt::test]
async fn should_perform_a_post_request() {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    actix_rt::spawn(async move {
        let api = "/director";
        let api_clone = api;

        HttpServer::new(move || {
            let url = format!("{}{}", api, "/host");
            let sender = sender.clone();
            App::new().app_data(Data::new(Arc::new(sender))).service(web::resource(&url).route(
                web::post().to(
                    move |body: Json<Value>, sender: Data<Arc<UnboundedSender<Value>>>| async move {
                        println!("Server received a call");
                        sender.send(body.into_inner()).unwrap();
                        ""
                    },
                ),
            ))
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

            actix_rt::spawn(async move {
                let executor = DirectorExecutor::new(config).unwrap();
                println!("Executor created");

                let mut action = Action::new("");
                action.payload.insert(
                    DIRECTOR_ACTION_NAME_KEY.to_owned(),
                    Value::String("create_host".to_owned()),
                );
                action.payload.insert(
                        DIRECTOR_ACTION_PAYLOAD_KEY.to_owned(),
                        json!(hashmap![
                            "object_type".to_owned() => Value::String("host".to_owned()),
                            "object_name".to_owned() => Value::String("my_host".to_owned()),
                            "address".to_owned() => Value::String("127.0.0.1".to_owned()),
                            "check_command".to_owned() => Value::String("hostalive".to_owned())
            ]),
                    );

                executor.execute(action.into()).await.unwrap();

                println!("DirectorApiClientMessage action sent");
            });

            Ok(server)
        })
        .expect("Can not bind to port 0")
        .run()
        .await
        .unwrap();
    });

    println!("actix System stopped");

    assert_eq!(
        Some(json!(hashmap![
            "object_type".to_owned() => Value::String("host".to_owned()),
            "object_name".to_owned() => Value::String("my_host".to_owned()),
            "address".to_owned() => Value::String("127.0.0.1".to_owned()),
            "check_command".to_owned() => Value::String("hostalive".to_owned())
        ])),
        receiver.recv().await
    );
}

#[tokio::test]
async fn should_return_object_already_existing_error_in_case_of_422_status_code() {
    // Arrange
    let server = MockServer::start();
    let server_response = "{\"error\": \"Trying to recreate icinga_host (\"some host\")\"}";

    server.mock(|when, then| {
        when.method(POST).path("/host");
        then.body(server_response).status(422);
    });

    let executor = DirectorExecutor::new(DirectorClientConfig {
        timeout_secs: None,
        username: "".to_owned(),
        password: "".to_owned(),
        disable_ssl_verification: true,
        server_api_url: server.url(""),
    })
    .unwrap();

    let mut action = Action::new("");
    action
        .payload
        .insert(DIRECTOR_ACTION_NAME_KEY.to_owned(), Value::String("create_host".to_owned()));
    action.payload.insert(
        DIRECTOR_ACTION_PAYLOAD_KEY.to_owned(),
        json!(hashmap![
                        "object_type".to_owned() => Value::String("host".to_owned()),
                        "object_name".to_owned() => Value::String("my_host".to_owned()),
                        "address".to_owned() => Value::String("127.0.0.1".to_owned()),
                        "check_command".to_owned() => Value::String("hostalive".to_owned())
        ]),
    );

    // Act
    let result = executor.execute(action.clone().into()).await;

    // Assert
    assert!(result.is_ok());
}
