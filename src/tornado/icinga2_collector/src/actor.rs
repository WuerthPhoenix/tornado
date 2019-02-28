use crate::config::Icinga2ClientConfig;
use crate::config::Stream;
use actix::Actor;
use actix::SyncContext;
use http::header;
use log::*;
use std::io::{BufRead, BufReader};
use tornado_collector_common::Collector;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common_api::Event;

pub struct Icinga2StreamActor<F: 'static + Fn(Event)> {
    pub icinga_config: Icinga2ClientConfig,
    pub collector: JMESPathEventCollector,
    pub stream_config: Stream,
    pub callback: F,
}

impl<F: 'static + Fn(Event)> Actor for Icinga2StreamActor<F> {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Starting Icinga2StreamActor with stream config: {:#?}", self.stream_config);

        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(self.icinga_config.disable_ssl_verification)
            .timeout(None)
            .build()
            // ToDo: to be investigated as part of TOR-56 (Resilience against downtime/restart of icinga2)
            .expect("Cannot create reqwest ClientBuilder");

        println!("Prepare request");

        let response = client
            .post(&self.icinga_config.server_api_url)
            .header(header::ACCEPT, "application/json")
            .basic_auth(
                self.icinga_config.username.clone(),
                Some(self.icinga_config.password.clone()),
            )
            .json(&self.stream_config)
            .send()
            // ToDo: to be investigated as part of TOR-56 (Resilience against downtime/restart of icinga2)
            .unwrap_or_else(|e| {
                panic!(
                    "Cannot perform POST request to {}. err: {}",
                    self.icinga_config.server_api_url, e
                )
            });

        println!("Got a response");

        let mut reader = BufReader::new(response);

        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(len) => {
                    if len == 0 {
                        // ToDo: to be investigated as part of TOR-56 (Resilience against downtime/restart of icinga2)
                        warn!("EOF received. Stopping Icinga2 collector.");
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
                Err(e) => error!(
                    "Error reading response from Icinga at url {}, Err: {}",
                    self.icinga_config.server_api_url, e
                ),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::Icinga2ClientConfig;
    use actix::prelude::*;
    use actix_web::Json;
    use actix_web::{server, App};
    use maplit::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
    use tornado_common_api::Value;
    //use tornado_common_logger::{setup_logger, LoggerConfig};

    #[test]
    fn should_perform_a_post_request() {
        //start_logger();
        let received = Arc::new(Mutex::new(None));

        let act_received = received.clone();
        System::run(move || {
            let api = "/v1/events";
            let api_clone = api.clone();

            server::new(move || {
                App::new().resource(api, move |r| {
                    r.with(move |body: Json<Stream>| {
                        info!("Server received a call with Stream: \n{:#?}", body.clone());
                        body
                    })
                })
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
                    };
                    let app_received = act_received.clone();
                    Icinga2StreamActor {
                        callback: move |event| {
                            info!("Callback called with Event: {:?}", event);
                            let mut message = app_received.lock().unwrap();
                            *message = Some(event);
                            System::current().stop();
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
            .start();
        });

        let event = received.lock().unwrap().clone().unwrap();
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
