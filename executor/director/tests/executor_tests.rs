use actix::prelude::*;
use actix_web::web::{Data, Json};
use actix_web::{web, App, HttpServer};
use maplit::*;
use std::sync::Arc;
use std::sync::Mutex;
use tornado_common_api::{Action, Value};
use tornado_executor_common::Executor;
use tornado_executor_director::config::DirectorClientConfig;
use tornado_executor_director::{
    DirectorExecutor, DIRECTOR_ACTION_NAME_KEY, DIRECTOR_ACTION_PAYLOAD_KEY,
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

            std::thread::spawn(move || {
                let mut executor = DirectorExecutor::new(config).unwrap();

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

                executor.execute(&action).unwrap();

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
