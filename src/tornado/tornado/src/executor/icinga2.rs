use actix::prelude::*;
use actix_web::client::{ClientConnector, ClientRequest};
use failure_derive::Fail;
use futures::future::Future;
use http::header;
use log::*;
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use serde_derive::{Deserialize, Serialize};
use std::time::Duration;

pub struct Icinga2ApiClientMessage {}

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
                icinga2_api_url: config.server_api_url,
                http_auth_header,
                client_connector,
            }
        })
    }
}

impl Handler<Icinga2ApiClientMessage> for Icinga2ApiClientActor {
    type Result = Result<(), Icinga2ApiClientActorError>;

    fn handle(&mut self, _msg: Icinga2ApiClientMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Icinga2ApiClientMessage - received new message");

        let request_body = "";

        let connector = self.client_connector.clone();

        actix::spawn(
            ClientRequest::post(&self.icinga2_api_url)
                .with_connector(connector)
                .header(header::ACCEPT, "application/json")
                .header(header::AUTHORIZATION, self.http_auth_header.as_str())
                .timeout(Duration::from_secs(10))
                .json(request_body)
                .unwrap()
                .send()
                .map_err(|err| panic!("Connection failed. Err: {}", err))
                .and_then(|response| {
                    info!("Response: {:?}", response);
                    /*
                                    response.body().map_err(|_| ()).map(|bytes| {
                                        println!("Body");
                                        println!("{:?}", bytes);
                                        ()
                                    });
                    */
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
    use actix_web::{server, App};
    use log::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    //    use tornado_common_logger::{LoggerConfig, setup_logger};

    #[test]
    fn should_perform_a_post_request() {
        // start_logger();
        let count = Arc::new(AtomicUsize::new(0));

        let act_count = count.clone();
        System::run(move || {
            let api = "/v1/events";
            let api_clone = api.clone();

            server::new(move || {
                let app_count = act_count.clone();
                App::new().resource(api, move |r| {
                    r.f(move |_h| {
                        info!("Server received a call");
                        app_count.fetch_add(1, Ordering::Relaxed);
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

                client_address.do_send(Icinga2ApiClientMessage {});

                Ok(server)
            })
            .expect("Can not bind to port 0")
            .start();
        });

        assert_eq!(count.load(Ordering::Relaxed), 1);
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
