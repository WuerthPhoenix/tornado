use crate::executor::{ApiClientActor, ApiClientActorError};
use actix::prelude::*;
use http::header;
use log::*;
use std::time::Duration;
use tornado_executor_icinga2::Icinga2Action;

pub struct Icinga2ApiClientMessage {
    pub message: Icinga2Action,
}

impl Message for Icinga2ApiClientMessage {
    type Result = Result<(), ApiClientActorError>;
}

impl Handler<Icinga2ApiClientMessage> for ApiClientActor {
    type Result = Result<(), ApiClientActorError>;

    fn handle(&mut self, msg: Icinga2ApiClientMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Icinga2ApiClientMessage - received new message");

        let url = format!("{}/{}", &self.server_api_url, msg.message.name);
        let http_auth_header = self.http_auth_header.to_owned();
        let client = self.client.clone();
        actix::spawn(async move {
            trace!("Icinga2ApiClientMessage - calling url: {}", url);

            let response = client
                .post(&url)
                .header(header::ACCEPT, "application/json")
                .header(header::AUTHORIZATION, http_auth_header.as_str())
                .timeout(Duration::from_secs(10))
                .json(&msg.message.payload)
                .send()
                .await
                .map_err(|err| {
                    error!("ApiClientActor - Icinga2 - Connection failed. Err: {}", err);
                    ApiClientActorError::ServerNotAvailableError { message: format!("{}", err) }
                })
                .expect("ApiClientActor - cannot connect to Icinga server");

            let response_status = response.status();

            let bytes = response
                .bytes()
                .await
                .map_err(|err| {
                    error!("ApiClientActor - Icinga2 - Cannot extract response body. Err: {}", err);
                    ApiClientActorError::ServerNotAvailableError { message: format!("{}", err) }
                })
                .expect("ApiClientActor - received an error from Icinga server");

            if !response_status.is_success() {
                error!("ApiClientActor - Icinga2 API returned an error. Response status: \n{:?}. Response body: {:?}", response_status, bytes)
            } else {
                debug!("ApiClientActor - Icinga2 API request completed successfully. Response body: {:?}", bytes);
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::executor::icinga2::Icinga2ApiClientMessage;
    use crate::executor::ApiClientActor;
    use crate::executor::ApiClientConfig;
    use actix::prelude::*;
    use actix_web::web::{Data, Json};
    use actix_web::{web, App, HttpServer};
    use maplit::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tornado_common_api::Value;
    use tornado_executor_icinga2::Icinga2Action;
    //    use tornado_common_logger::{LoggerConfig, setup_logger};

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

                let config = ApiClientConfig {
                    server_api_url: url,
                    disable_ssl_verification: true,
                    password: "".to_owned(),
                    username: "".to_owned(),
                };
                let client_address = ApiClientActor::start_new(config);

                println!("ApiClientActor created");

                client_address.do_send(Icinga2ApiClientMessage {
                    message: Icinga2Action {
                        name: "icinga2-api-action".to_owned(),
                        payload: hashmap![
                            "filter".to_owned() => Value::Text("my_service".to_owned())
                        ],
                    },
                });

                println!("Icinga2ApiClientActor message sent");

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
}
