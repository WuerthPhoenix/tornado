use actix::prelude::*;
use http::header;
use log::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tornado_executor_common::ExecutorError;
use tornado_executor_director::DirectorAction;

pub struct DirectorApiClientMessage {
    pub message: DirectorAction,
}

impl Message for DirectorApiClientMessage {
    type Result = Result<(), DirectorApiClientActorError>;
}

#[derive(Error, Debug)]
pub enum DirectorApiClientActorError {
    #[error("ServerNotAvailableError: cannot connect to [{message}]")]
    ServerNotAvailableError { message: String },
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DirectorClientConfig {
    /// The complete URL of the Director APIs
    pub server_api_url: String,

    /// Username used to connect to the Director APIs
    pub username: String,

    /// Password used to connect to the Director APIs
    pub password: String,

    /// If true, the client will not verify the SSL certificate
    pub disable_ssl_verification: bool,
}

pub struct DirectorApiClientActor {
    director_api_url: String,
    http_auth_header: String,
    client: Client,
}

impl Actor for DirectorApiClientActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("DirectorApiClientActor started.");
    }
}

impl DirectorApiClientActor {
    pub fn start_new(config: DirectorClientConfig) -> Addr<Self> {
        DirectorApiClientActor::create(move |_ctx: &mut Context<DirectorApiClientActor>| {
            let auth = format!("{}:{}", config.username, config.password);
            let http_auth_header = format!("Basic {}", base64::encode(&auth));

            let mut client_builder = Client::builder().use_native_tls();
            if config.disable_ssl_verification {
                client_builder = client_builder.danger_accept_invalid_certs(true)
            }

            let client = client_builder
                .build()
                .map_err(|err| ExecutorError::ConfigurationError {
                    message: format!("Error while building reqwest client. Err: {}", err),
                })
                .unwrap();

            DirectorApiClientActor {
                director_api_url: config.server_api_url,
                http_auth_header,
                client,
            }
        })
    }
}

impl Handler<DirectorApiClientMessage> for DirectorApiClientActor {
    type Result = Result<(), DirectorApiClientActorError>;

    fn handle(&mut self, msg: DirectorApiClientMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("DirectorApiClientMessage - received new message");

        let mut url =
            format!("{}/{}", &self.director_api_url, msg.message.name.to_director_api_subpath());

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
                    error!("DirectorApiClientActor - Connection failed. Err: {}", err);
                    DirectorApiClientActorError::ServerNotAvailableError {
                        message: format!("{:?}", err),
                    }
                })
                .expect("DirectorApiClientActor - cannot connect to Director server");

            let response_status = response.status();

            let bytes = response
                .bytes()
                .await
                .map_err(|err| {
                    error!("DirectorApiClientActor - Cannot extract response body. Err: {}", err);
                    DirectorApiClientActorError::ServerNotAvailableError {
                        message: format!("{:?}", err),
                    }
                })
                .expect("DirectorApiClientActor - received an error from Director server");

            if !response_status.is_success() {
                error!("DirectorApiClientActor - Director API returned an error. Response status: \n{:?}. Response body: {:?}", response_status, bytes)
            } else {
                debug!("DirectorApiClientActor - Director API request completed successfully. Response body: {:?}", bytes);
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::executor::director::{
        DirectorApiClientActor, DirectorApiClientMessage, DirectorClientConfig,
    };
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

                let config = DirectorClientConfig {
                    server_api_url: url,
                    disable_ssl_verification: true,
                    password: "".to_owned(),
                    username: "".to_owned(),
                };
                let client_address = DirectorApiClientActor::start_new(config);

                println!("DirectorApiClientActor created");

                client_address.do_send(DirectorApiClientMessage {
                    message: DirectorAction {
                        name: DirectorActionName::CreateHost,
                        payload: hashmap![
                            "object_type".to_owned() => Value::Text("host".to_owned()),
                            "object_name".to_owned() => Value::Text("my_host".to_owned()),
                            "address".to_owned() => Value::Text("127.0.0.1".to_owned()),
                            "check_command".to_owned() => Value::Text("hostalive".to_owned())
                        ],
                        live_creation: false
                    },
                });

                println!("DirectorApiClientActor message sent");

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
