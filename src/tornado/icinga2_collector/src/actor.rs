use tornado_collector_jmespath::JMESPathEventCollector;
use actix::Actor;
use actix::SyncContext;
use http::header;
use log::*;
use std::io::{BufRead, BufReader};
use std::time::Duration;
use crate::config::Stream;
use crate::config::Icinga2ClientConfig;
use tornado_collector_common::Collector;

pub struct Icinga2StreamActor {
    pub icinga_config: Icinga2ClientConfig,
    pub collector: JMESPathEventCollector,
    pub stream_config: Stream
}

impl Actor for Icinga2StreamActor {
    type Context = SyncContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting Icinga2StreamActor with stream config: {:#?}", self.stream_config);

        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(self.icinga_config.disable_ssl_verification)
            .timeout(None)
            .build()
            .unwrap();

        println!("Prepare request");

        let response = client
            .post(&self.icinga_config.server_api_url)
            .header(header::ACCEPT, "application/json")
            .basic_auth(self.icinga_config.username.clone(), Some(self.icinga_config.password.clone()))
            .json(&self.stream_config)
            .send()
            .unwrap();

        println!("Got a response");

        let mut reader = BufReader::new(response);

        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(len) => {
                    if len == 0 {
                        warn!("EOF received. Stopping Icinga2 collector.");
                        //system.stop();
                    } else {
                        info!("Received line: {}", line);
                        let event = self.collector.to_event(&line).unwrap();
                        info!("Generated event: \n{}", serde_json::to_string_pretty(&event).unwrap());
                    }
                },
                Err(e) => {
                    error!("Error reading response from Icinga at url {}, Err: {}", self.icinga_config.server_api_url, e)
                }
            }
        }
    }
}