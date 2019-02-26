use actix::prelude::*;
use actix_web::client::{ClientConnector, ClientRequest};
use actix_web::HttpMessage;
use failure_derive::Fail;
use futures::future::Future;
use http::header;
use log::*;
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use serde_derive::{Deserialize, Serialize};
use std::time::Duration;
use tornado_executor_icinga2::Icinga2Action;

pub struct Icinga2ApiClientMessage {
    pub message: Icinga2Action,
}

impl Message for Icinga2ApiClientMessage {
    type Result = Result<(), Icinga2ApiClientActorError>;
}

#[derive(Fail, Debug)]
pub enum Icinga2ApiClientActorError {
    #[fail(display = "ServerNotAvailableError: cannot connect to [{}]", message)]
    ServerNotAvailableError { message: String },
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Icinga2ClientConfig {
    /// The complete URL of the Icinga2 APIs
    pub server_api_url: String,

    /// Username used to connect to the Icinga2 APIs
    pub username: String,

    /// Password used to connect to the Icinga2 APIs
    pub password: String,

    /// If true, the client will not verify the SSL certificate
    pub disable_ssl_verification: bool,
}

pub struct Icinga2ApiClientActor {
    //username: String,
    //password: String,
    icinga2_api_url: String,
    http_auth_header: String,
    client_connector: Addr<ClientConnector>,
}

impl Actor for Icinga2ApiClientActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Icinga2ApiClientActor started.");
    }
}

impl Icinga2ApiClientActor {
    pub fn start_new(config: Icinga2ClientConfig) -> Addr<Self> {
        Icinga2ApiClientActor::create(move |_ctx: &mut Context<Icinga2ApiClientActor>| {
            let auth = format!("{}:{}", config.username, config.password);
            let http_auth_header = format!("Basic {}", base64::encode(&auth));

            let mut ssl_conn_builder = SslConnector::builder(SslMethod::tls()).unwrap();
            if config.disable_ssl_verification {
                ssl_conn_builder.set_verify(SslVerifyMode::NONE);
            }
            let ssl_connector = ssl_conn_builder.build();
            let client_connector = ClientConnector::with_connector(ssl_connector).start();

            Icinga2ApiClientActor {
                //username: config.username,
                //password: config.password,
                icinga2_api_url: config.server_api_url,
                http_auth_header,
                client_connector,
            }
        })
    }
}

impl Handler<Icinga2ApiClientMessage> for Icinga2ApiClientActor {
    type Result = Result<(), Icinga2ApiClientActorError>;

    fn handle(&mut self, msg: Icinga2ApiClientMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Icinga2ApiClientMessage - received new message");

        let connector = self.client_connector.clone();
        let url = &format!("{}/{}", &self.icinga2_api_url, msg.message.name);

        debug!("Icinga2ApiClientMessage - calling url: {}", url);

        actix::spawn(
            ClientRequest::post(url)
                .with_connector(connector)
                .header(header::ACCEPT, "application/json")
                .header(header::AUTHORIZATION, self.http_auth_header.as_str())
                .timeout(Duration::from_secs(10))
                .json(msg.message.payload)
                .unwrap()
                .send()
                .map_err(|err| error!("Connection failed. Err: {}", err))
                .and_then(|response| {
                    actix::spawn(response.body().map_err(|_| ()).map(move |bytes| {
                        if !response.status().is_success() {
                            error!("Icinga2 API returned an error. Response: \n{:#?}. Response body: {:#?}", response, bytes)
                        } else {
                            debug!("Icinga2 API request completed successfully. Response body: {:?}", bytes);
                        }
                    }));
                    Ok(())
                }),
        );

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::executor::icinga2::Icinga2ApiClientActor;
    use crate::executor::icinga2::Icinga2ApiClientMessage;
    use crate::executor::icinga2::Icinga2ClientConfig;
    use actix::prelude::*;
    use actix_web::Json;
    use actix_web::{server, App};
    use log::*;
    use maplit::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tornado_common_api::Value;
    use tornado_executor_icinga2::Icinga2Action;
    //    use tornado_common_logger::{LoggerConfig, setup_logger};

    #[test]
    fn should_perform_a_post_request() {
        // start_logger();
        let received = Arc::new(Mutex::new(None));

        let act_received = received.clone();
        System::run(move || {
            let api = "/v1/events";
            let api_clone = api.clone();

            server::new(move || {
                let app_received = act_received.clone();
                App::new().resource(&format!("{}{}", api, "/icinga2-api-action"), move |r| {
                    r.with(move |body: Json<Value>| {
                        info!("Server received a call");
                        let mut message = app_received.lock().unwrap();
                        *message = Some(body.into_inner());
                        System::current().stop();
                        ""
                    })
                })
            })
            .bind("127.0.0.1:0")
            .and_then(|server| {
                let server_port = server.addrs()[0].port();

                let url = format!("http://127.0.0.1:{}{}", server_port, api_clone);
                warn!("Client connecting to: {}", url);

                let config = Icinga2ClientConfig {
                    server_api_url: url,
                    disable_ssl_verification: true,
                    password: "".to_owned(),
                    username: "".to_owned(),
                };
                let client_address = Icinga2ApiClientActor::start_new(config);

                client_address.do_send(Icinga2ApiClientMessage {
                    message: Icinga2Action {
                        name: "icinga2-api-action".to_owned(),
                        payload: hashmap![
                            "filter".to_owned() => Value::Text("my_service".to_owned())
                        ],
                    },
                });

                Ok(server)
            })
            .expect("Can not bind to port 0")
            .start();
        });

        assert_eq!(
            Some(Value::Map(hashmap![
                "filter".to_owned() => Value::Text("my_service".to_owned())
            ])),
            *received.lock().unwrap()
        );
    }
    /*
    fn start_logger() {
        println!("Init logger");

        let conf = LoggerConfig {
            level: String::from("info"),
            stdout_output: true,
            file_output_path: None,
        };
        setup_logger(&conf).unwrap();
    }
    */
}
