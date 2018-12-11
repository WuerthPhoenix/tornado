extern crate tornado_collector_common;
extern crate tornado_collector_json;
extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_engine_matcher;
extern crate tornado_executor_common;
extern crate tornado_executor_logger;
extern crate tornado_network_common;
extern crate tornado_network_simple;

#[macro_use]
extern crate log;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate structopt;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_uds;

use futures::sync::mpsc;
use std::fs;
use std::sync::Arc;
use std::thread;
use tokio::prelude::*;
use tokio::runtime::Runtime;
use tornado_collector_common::Collector;
use tornado_collector_json::JsonEventCollector;
use tornado_common_logger::setup_logger;
use tornado_engine_matcher::config::Rule;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tornado_engine_matcher::matcher::Matcher;
use tornado_network_simple::SimpleEventBus;

mod config;
mod io;

fn main() {
    let conf = config::Conf::build();
    setup_logger(&conf.logger).unwrap();

    // Load rules from fs
    let config_rules = read_rules_from_config(&conf.io.rules_dir);

    // Start matcher & dispatcher
    let matcher = Arc::new(Matcher::new(&config_rules).unwrap());
    let collector = Arc::new(JsonEventCollector::new());

    // Configure action dispatcher
    let event_bus = {
        let mut event_bus = SimpleEventBus::new();

        /*
        let executor = LoggerExecutor::new();
        event_bus.subscribe_to_action(
            "Logger",
            Box::new(move |action| match executor.execute(&action) {
                Ok(_) => {}
                Err(e) => error!("Cannot log action: {}", e),
            }),
        );
        */
        Arc::new(event_bus)
    };

    let mut runtime = Runtime::new().unwrap();

    let (tx, rx) = mpsc::unbounded::<String>();

    runtime.spawn(rx.for_each(move |line| {
        debug!("Client - Thread {:?} - Received line {}", thread::current().name(), line);

        match collector.to_event(&line) {
            Ok(event) => {
                let matcher_clone = matcher.clone();
                let event_bus_clone = event_bus.clone();
                tokio::spawn(future::lazy(move || {
                    debug!(
                        "Client - Thread {:?} - Got event!! span matcher thread",
                        thread::current().name()
                    );
                    let processed_event = matcher_clone.process(event);
                    match Dispatcher::new(event_bus_clone)
                        .unwrap()
                        .dispatch_actions(processed_event)
                    {
                        Ok(_) => {}
                        Err(e) => error!("Cannot dispatch action: {}", e),
                    };
                    Ok(())
                }));
            }
            Err(e) => error!(
                "JsonReaderActor - {:?} - Cannot unmarshal event from json: {}",
                thread::current().name(),
                e
            ),
        };

        Ok(())
    }));

    let server = io::uds::start_uds_socket(conf.io.uds_path, tx);
    runtime
        .block_on(server.map_err(|e| panic!("err={:?}", e)))
        .expect("Tokio runtime should start");
}

fn read_rules_from_config(path: &str) -> Vec<Rule> {
    let paths = fs::read_dir(path).unwrap();
    let mut rules = vec![];

    for path in paths {
        let filename = path.unwrap().path();
        info!("Loading rule from file: [{}]", filename.display());
        let rule_body = fs::read_to_string(&filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename.display()));
        trace!("Rule body: \n{}", rule_body);
        rules.push(Rule::from_json(&rule_body).unwrap());
    }

    info!("Loaded {} rule(s) from [{}]", rules.len(), path);

    rules
}
