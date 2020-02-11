use crate::config::{Icinga2ClientConfig, Stream};
use crate::error::Icinga2CollectorError;
use actix::prelude::*;
use http::header;
use log::*;
use reqwest::Client;
use std::io::{BufRead, BufReader};
use std::{thread, time};
use tornado_collector_common::Collector;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common_api::Event;

pub struct Icinga2StreamActor<F: 'static + Fn(Event) + Unpin> {
    pub icinga_config: Icinga2ClientConfig,
    pub collector: JMESPathEventCollector,
    pub stream_config: Stream,
    pub callback: F,
}

impl<F: 'static + Fn(Event) + Unpin> Icinga2StreamActor<F> {
    fn start_polling(&mut self, client: &Client) -> Result<(), Icinga2CollectorError> {
        info!("Starting Event Stream call to Icinga2");

        let mut response = client
            .post(&self.icinga_config.server_api_url)
            .header(header::ACCEPT, "application/json")
            .basic_auth(
                self.icinga_config.username.clone(),
                Some(self.icinga_config.password.clone()),
            )
            .json(&self.stream_config)
            .send()
            .map_err(|e| Icinga2CollectorError::CannotPerformHttpRequest {
                message: format!(
                    "Cannot perform POST request to {}. err: {}",
                    self.icinga_config.server_api_url, e
                ),
            })?;

        if !response.status().is_success() {
            let body = match response.text() {
                Ok(body) => body,
                _ => "".to_owned(),
            };

            return Err(Icinga2CollectorError::CannotPerformHttpRequest {
                message: format!(
                    "Failed response returned from Icinga2, Response status: {:?} - body: {}",
                    response.status(),
                    body
                ),
            });
        }

        let mut reader = BufReader::new(response);

        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(len) => {
                    if len == 0 {
                        warn!("EOF received. Stopping Icinga2 collector.");
                        return Err(Icinga2CollectorError::UnexpectedEndOfHttpRequest);
                    } else {
                        debug!("Received line: {}", line);
                        match self.collector.to_event(&line) {
                            Ok(event) => (self.callback)(event),
                            Err(e) => {
                                error!("Error processing Icinga2 response: [{}], Err: {}", line, e)
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(Icinga2CollectorError::CannotPerformHttpRequest {
                        message: format!(
                            "Error reading response from Icinga at url {}, Err: {}",
                            self.icinga_config.server_api_url, e
                        ),
                    });
                }
            }
        }
    }
}

impl<F: 'static + Fn(Event) + Unpin> Actor for Icinga2StreamActor<F> {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Starting Icinga2StreamActor with stream config: {:?}", self.stream_config);

        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(self.icinga_config.disable_ssl_verification)
            .timeout(None)
            .build()
            .unwrap_or_else(|e| {
                System::current().stop();
                panic!("Impossible to create a connection to the Icinga2 server. Err: {}", e)
            });

        loop {
            if let Err(e) = self.start_polling(&client) {
                error!("Client connection to Icinga2 Server dropped. Err: {}", e);
                info!(
                    "Attempting a new connection in {} ms",
                    self.icinga_config.sleep_ms_between_connection_attempts
                );

                let sleep_millis = time::Duration::from_millis(
                    self.icinga_config.sleep_ms_between_connection_attempts,
                );
                thread::sleep(sleep_millis);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Icinga2ClientConfig;
    use actix_web::web::Json;
    use actix_web::{web, App, HttpServer};
    use maplit::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
    use tornado_common_api::Value;
    //use tornado_common_logger::{setup_logger, LoggerConfig};

    #[test]
    fn should_perform_a_post_request() {
        //start_logger();
        let received = Arc::new(Mutex::new(vec![]));

        let act_received = received.clone();

        let sys = actix_rt::System::new("basic-example");

        let api = "/v1/events";
        let api_clone = api.clone();

        HttpServer::new(move || {
            App::new().service(web::resource(api).route(web::post().to(
                move |body: Json<Stream>| async {
                    info!("Server received a call with Stream: \n{:?}", body.clone());
                    body
                },
            )))
        })
        .bind("127.0.0.1:0")
        .and_then(|server| {
            let server_port = server.addrs()[0].port();

            let url = format!("http://127.0.0.1:{}{}", server_port, api_clone);
            warn!("Client connecting to: {}", url);

            SyncArbiter::start(1, move || {
                let icinga_config = Icinga2ClientConfig {
                    server_api_url: url.clone(),
                    disable_ssl_verification: true,
                    password: "".to_owned(),
                    username: "".to_owned(),
                    sleep_ms_between_connection_attempts: 0,
                };
                let app_received = act_received.clone();
                Icinga2StreamActor {
                    callback: move |event| {
                        info!("Callback called with Event: {:?}", event);
                        let mut message = app_received.lock().unwrap();
                        message.push(event);

                        // The actor tries to establish a long polling connection to the server;
                        // however, the server used in this test drops the connection after each response.
                        // We check that we added three messages into the mutek, if this succeeds,
                        // it means that the actor was successfully restarted after each connection drop, so the client
                        // is correctly handling dropped connections.

                        if message.len() > 2 {
                            System::current().stop();
                        }
                    },
                    collector: JMESPathEventCollector::build(JMESPathEventCollectorConfig {
                        event_type: "test".to_owned(),
                        payload: hashmap![
                            "response".to_owned() => Value::Text("${@}".to_owned())
                        ],
                    })
                    .unwrap(),
                    icinga_config,
                    stream_config: Stream {
                        filter: Some("filter".to_owned()),
                        queue: "queue_name".to_owned(),
                        types: vec![],
                    },
                }
            });

            Ok(server)
        })
        .expect("Can not bind to port 0")
        .run();

        sys.run().unwrap();

        let events = received.lock().unwrap().clone();
        assert_eq!(3, events.len());

        events.iter().for_each(|event| {
            assert_eq!("test".to_owned(), event.event_type);
            assert_eq!(
                "queue_name".to_owned(),
                event
                    .payload
                    .get("response")
                    .unwrap()
                    .get_from_map("queue")
                    .unwrap()
                    .get_text()
                    .unwrap()
            )
        });
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
