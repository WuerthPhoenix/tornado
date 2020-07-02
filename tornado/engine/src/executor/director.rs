use crate::executor::{ApiClientActor, ApiClientActorError};
use actix::prelude::*;
use http::header;
use log::*;
use std::time::Duration;
use tornado_executor_director::DirectorAction;

pub struct DirectorApiClientMessage {
    pub message: DirectorAction,
}

impl Message for DirectorApiClientMessage {
    type Result = Result<(), ApiClientActorError>;
}

impl Handler<DirectorApiClientMessage> for ApiClientActor {
    type Result = Result<(), ApiClientActorError>;

    fn handle(&mut self, msg: DirectorApiClientMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("DirectorApiClientMessage - received new message");

        let mut url =
            format!("{}/{}", &self.server_api_url, msg.message.name.to_director_api_subpath());

        trace!(
            "DirectorApiClientMessage - icinga2 live creation is set to: {}",
            msg.message.live_creation
        );
        if msg.message.live_creation {
            url.push_str("?live-creation=true");
        }
        let http_auth_header = self.http_auth_header.to_owned();
        let client = self.client.clone();
        actix::spawn(async move {
            trace!("DirectorApiClientMessage - calling url: {}", url);

            let response = client
                .post(&url)
                .header(header::ACCEPT, "application/json")
                .header(header::AUTHORIZATION, http_auth_header.as_str())
                .timeout(Duration::from_secs(10))
                .json(&msg.message.payload)
                .send()
                .await
                .map_err(|err| {
                    error!("ApiClientActor - Director - Connection failed. Err: {}", err);
                    ApiClientActorError::ServerNotAvailableError { message: format!("{:?}", err) }
                })
                .expect("ApiClientActor - Director - cannot connect to Director server");

            let response_status = response.status();

            let bytes = response
                .bytes()
                .await
                .map_err(|err| {
                    error!(
                        "ApiClientActor - Director - Cannot extract response body. Err: {}",
                        err
                    );
                    ApiClientActorError::ServerNotAvailableError { message: format!("{:?}", err) }
                })
                .expect("ApiClientActor - received an error from Director server");

            if !response_status.is_success() {
                error!("ApiClientActor - Director API returned an error. Response status: \n{:?}. Response body: {:?}", response_status, bytes)
            } else {
                debug!("ApiClientActor - Director API request completed successfully. Response body: {:?}", bytes);
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::executor::director::DirectorApiClientMessage;
    use crate::executor::{ApiClientActor, ApiClientConfig};
    use actix::prelude::*;
    use actix_web::web::{Data, Json};
    use actix_web::{web, App, HttpServer};
    use maplit::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tornado_common_api::Value;
    use tornado_executor_director::{DirectorAction, DirectorActionName};

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

                let config = ApiClientConfig {
                    server_api_url: url,
                    disable_ssl_verification: true,
                    password: "".to_owned(),
                    username: "".to_owned(),
                };
                let client_address = ApiClientActor::start_new(config);

                println!("ApiClientActor for Director created");

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

                println!("DirectorApiClientMessage message sent");

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
}
