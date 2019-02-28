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
            .unwrap_or_else(|e| panic!("Cannot perform POST request to {}. err: {}", self.icinga_config.server_api_url, e));

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
                    //system.stop();
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
