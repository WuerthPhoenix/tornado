use crate::config::{Icinga2ClientConfig, Stream};
use crate::error::Icinga2CollectorError;
use futures::stream::TryStreamExt;
use log::*;
use reqwest::{header, Client};
use std::time;
use tokio::io::AsyncBufReadExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tornado_collector_common::Collector;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common_api::Event;

pub struct Icinga2StreamConnector<F: 'static + Fn(Event) + Unpin> {
    pub icinga_config: Icinga2ClientConfig,
    pub collector: JMESPathEventCollector,
    pub stream_config: Stream,
    pub callback: F,
}

impl<F: 'static + Fn(Event) + Unpin> Icinga2StreamConnector<F> {
    async fn start_polling(&self, client: &Client) -> Result<(), Icinga2CollectorError> {
        info!("Starting Event Stream call to Icinga2");

        let response = client
            .post(&self.icinga_config.server_api_url)
            .header(header::ACCEPT, "application/json")
            .basic_auth(
                self.icinga_config.username.clone(),
                Some(self.icinga_config.password.clone()),
            )
            .json(&self.stream_config)
            .send()
            .await
            .map_err(|e| Icinga2CollectorError::CannotPerformHttpRequest {
                message: format!(
                    "Cannot perform POST request to {}. err: {}",
                    self.icinga_config.server_api_url, e
                ),
            })?;

        let response_status = response.status();
        if !response_status.is_success() {
            let body = match response.text().await {
                Ok(body) => body,
                _ => "".to_owned(),
            };

            return Err(Icinga2CollectorError::CannotPerformHttpRequest {
                message: format!(
                    "Failed response returned from Icinga2, Response status: {:?} - body: {}",
                    response_status, body
                ),
            });
        }

        let reader = {
            // Convert the body of the response into a futures::io::Stream.
            let response = response.bytes_stream();

            // Convert the stream into an futures::io::AsyncRead.
            // We must first convert the reqwest::Error into an futures::io::Error.
            let response = response
                .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
                .into_async_read();

            // Convert the futures::io::AsyncRead into a tokio::io::AsyncRead.
            let response = response.compat();

            tokio::io::BufReader::new(response)
        };

        let mut lines = reader.lines();

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    debug!("Received line: {}", line);
                    match self.collector.to_event(&line) {
                        Ok(event) => (self.callback)(event),
                        Err(e) => {
                            error!("Error processing Icinga2 response: [{}], Err: {:?}", line, e)
                        }
                    }
                }
                Ok(None) => {
                    warn!("EOF received. Stopping Icinga2 collector.");
                    return Err(Icinga2CollectorError::UnexpectedEndOfHttpRequest);
                }
                Err(e) => {
                    return Err(Icinga2CollectorError::CannotPerformHttpRequest {
                        message: format!(
                            "Error reading response from Icinga at url {}, Err: {:?}",
                            self.icinga_config.server_api_url, e
                        ),
                    });
                }
            }
        }
    }

    pub async fn start_polling_icinga(&self) -> Result<(), Icinga2CollectorError> {
        info!("Starting Icinga2StreamConnector with stream config: {:?}", self.stream_config);

        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(self.icinga_config.disable_ssl_verification)
            .build()
            .map_err(|e| Icinga2CollectorError::IcingaConnectionError {
                message: format!(
                    "Cannot connect to Icinga at url {}, Err: {:?}",
                    self.icinga_config.server_api_url, e
                ),
            })?;

        loop {
            if let Err(e) = self.start_polling(&client).await {
                error!("Client connection to Icinga2 Server dropped. Err: {:?}", e);
                info!(
                    "Attempting a new connection in {} ms",
                    self.icinga_config.sleep_ms_between_connection_attempts
                );

                let sleep_millis = time::Duration::from_millis(
                    self.icinga_config.sleep_ms_between_connection_attempts,
                );
                tokio::time::sleep(sleep_millis).await;
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
    use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
    use tornado_common_api::Value;
    use tornado_common_api::ValueGet;
    //use tornado_common_logger::{setup_logger, LoggerConfig};

    #[actix_rt::test]
    async fn should_perform_a_post_request() {
        //start_logger();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        let api = "/v1/events";
        let api_clone = api.clone();

        actix::spawn(async move {
            HttpServer::new(move || {
                App::new().service(web::resource(api).route(web::post().to(
                    move |body: Json<Stream>| async {
                        info!("Server received a call with Stream: \n{:?}", &body);
                        body
                    },
                )))
            })
            .bind("127.0.0.1:0")
            .and_then(|server| {
                let server_port = server.addrs()[0].port();

                let url = format!("http://127.0.0.1:{}{}", server_port, api_clone);
                warn!("Client connecting to: {}", url);

                actix::spawn(async move {
                    let icinga_config = Icinga2ClientConfig {
                        server_api_url: url.clone(),
                        disable_ssl_verification: true,
                        password: "".to_owned(),
                        username: "".to_owned(),
                        sleep_ms_between_connection_attempts: 0,
                    };
                    let sender = sender.clone();
                    let icinga_poll = Icinga2StreamConnector {
                        callback: move |event| {
                            info!("Callback called with Event: {:?}", event);
                            sender.send(event).unwrap();

                            // The actor tries to establish a long polling connection to the server;
                            // however, the server used in this test drops the connection after each response.
                            // We check that we added three messages into the mutek, if this succeeds,
                            // it means that the actor was successfully restarted after each connection drop, so the client
                            // is correctly handling dropped connections.
                        },
                        collector: JMESPathEventCollector::build(JMESPathEventCollectorConfig {
                            event_type: "test".to_owned(),
                            payload: hashmap![
                                "response".to_owned() => Value::String("${@}".to_owned())
                            ],
                        })
                        .unwrap(),
                        icinga_config,
                        stream_config: Stream {
                            filter: Some("filter".to_owned()),
                            queue: "queue_name".to_owned(),
                            types: vec![],
                        },
                    };
                    icinga_poll.start_polling_icinga().await.unwrap();
                });

                Ok(server)
            })
            .expect("Can not bind to port 0")
            .run()
            .await
            .unwrap();
        });

        let mut events = vec![];
        events.push(receiver.recv().await.unwrap());
        events.push(receiver.recv().await.unwrap());
        events.push(receiver.recv().await.unwrap());
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
