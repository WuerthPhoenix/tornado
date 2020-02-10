use actix::prelude::*;
use actix_web::client::{Client, ClientBuilder, Connector};
use failure_derive::Fail;
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
    client: Client,
}

impl Actor for Icinga2ApiClientActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("Icinga2ApiClientActor started.");
    }
}

impl Icinga2ApiClientActor {
    pub fn start_new(config: Icinga2ClientConfig) -> Addr<Self> {
        Icinga2ApiClientActor::create(move |_ctx: &mut Context<Icinga2ApiClientActor>| {
            let auth = format!("{}:{}", config.username, config.password);
            let http_auth_header = format!("Basic {}", base64::encode(&auth));

            // Build client connector with native-tls
            // Simpler cross platform build, but currently not supported by actix-web 1.0
            /*
            let client_connector = {
                let mut ssl_conn_builder = native_tls::TlsConnector::builder();

                if config.disable_ssl_verification {
                    ssl_conn_builder.danger_accept_invalid_certs(true);
                }
                let ssl_connector = ssl_conn_builder.build().unwrap();

                ClientConnector::with_connector(tokio_tls::TlsConnector::from(ssl_connector)).start()
            };
            */

            // Build client connector with OpenSSL
            let client_connector = {
                let mut ssl_conn_builder = SslConnector::builder(SslMethod::tls()).unwrap();
                if config.disable_ssl_verification {
                    ssl_conn_builder.set_verify(SslVerifyMode::NONE);
                }
                let ssl_connector = ssl_conn_builder.build();
                Connector::new().ssl(ssl_connector).finish()
            };

            let client = ClientBuilder::new().connector(client_connector).finish();

            Icinga2ApiClientActor {
                //username: config.username,
                //password: config.password,
                icinga2_api_url: config.server_api_url,
                http_auth_header,
                client,
            }
        })
    }
}

impl Handler<Icinga2ApiClientMessage> for Icinga2ApiClientActor {
    type Result = ResponseFuture<Result<(), Icinga2ApiClientActorError>>;

    fn handle(&mut self, msg: Icinga2ApiClientMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Icinga2ApiClientMessage - received new message");

        let url = format!("{}/{}", &self.icinga2_api_url, msg.message.name);
        let http_auth_header = self.http_auth_header.to_owned();
        let client = self.client.clone();
        Box::pin(
            async move {

                trace!("Icinga2ApiClientMessage - calling url: {}", url);

                let mut response = client.post(url)
//                .with_connector(connector)
                .header(header::ACCEPT, "application/json")
                .header(header::AUTHORIZATION, http_auth_header.as_str())
                .timeout(Duration::from_secs(10))
                .send_json(&msg.message.payload)
                .await
                .map_err(|err| {
                    error!("Icinga2ApiClientActor - Connection failed. Err: {}", err);
                    Icinga2ApiClientActorError::ServerNotAvailableError {message: format!("{}", err)}
                })?;

                response.body().await
                    .map_err(|err| {
                        error!("Icinga2ApiClientActor - Cannot extract response body. Err: {}", err);
                        Icinga2ApiClientActorError::ServerNotAvailableError {message: format!("{}", err)}
                    })
                    .map(move |bytes| {
                    if !response.status().is_success() {
                        error!("Icinga2ApiClientActor - Icinga2 API returned an error. Response: \n{:?}. Response body: {:?}", response, bytes)
                    } else {
                        debug!("Icinga2ApiClientActor - Icinga2 API request completed successfully. Response body: {:?}", bytes);
                    }
                })?;

                Ok(())

            })


    }
}

#[cfg(test)]
mod test {
    use crate::executor::icinga2::Icinga2ApiClientActor;
    use crate::executor::icinga2::Icinga2ApiClientMessage;
    use crate::executor::icinga2::Icinga2ClientConfig;
    use actix::prelude::*;
    use actix_web::web::Json;
    use actix_web::{web, App, HttpServer};
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

            HttpServer::new(move || {
                let app_received = act_received.clone();
                let url = format!("{}{}", api, "/icinga2-api-action");

                App::new().service(web::resource(&url).route(web::post().to(
                    move |body: Json<Value>| {
                        info!("Server received a call");
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
        })
        .unwrap();

        assert_eq!(
            Some(Value::Map(hashmap![
                "filter".to_owned() => Value::Text("my_service".to_owned())
            ])),
            *received.lock().unwrap()
        );
    }

}
