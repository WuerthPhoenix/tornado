extern crate tornado_collector_common;
extern crate tornado_collector_json;
extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_engine_matcher;
extern crate tornado_executor_common;
extern crate tornado_executor_logger;
extern crate tornado_network_common;
extern crate tornado_network_simple;

extern crate actix;
extern crate config as config_rs;
extern crate futures;
#[macro_use]
extern crate log;
extern crate num_cpus;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate structopt;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_uds;

pub mod collector;
pub mod config;
pub mod engine;
pub mod executor;
pub mod reader;

use actix::prelude::*;
use collector::JsonReaderActor;
use engine::MatcherActor;
use executor::ExecutorActor;
use reader::uds::listen_to_uds_socket;
use std::fs;
use std::sync::Arc;
use tornado_common_logger::setup_logger;
use tornado_engine_matcher::config::Rule;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tornado_engine_matcher::matcher::Matcher;
use tornado_executor_common::Executor;
use tornado_executor_logger::LoggerExecutor;
use tornado_network_common::EventBus;
use tornado_network_simple::SimpleEventBus;

fn main() {
    let conf = config::Conf::new().expect("Should read the configuration");

    setup_logger(&conf.logger).unwrap();

    // Load rules from fs
    let config_rules = read_rules_from_config(&conf.io.json_rules_path);

    // Start matcher & dispatcher
    let matcher = Arc::new(Matcher::new(&config_rules).unwrap());
    //let event_bus = Arc::new(SimpleEventBus::new());
    //let dispatcher = Arc::new(Dispatcher::new(event_bus.clone()).unwrap());

    // start system
    System::run(move || {
        let cpus = num_cpus::get();
        info!("Available CPUs: {}", cpus);

        // Configure action dispatcher
        let event_bus = {
            let mut event_bus = SimpleEventBus::new();

            let executor = LoggerExecutor::new();
            event_bus.subscribe_to_action(
                "Logger",
                Box::new(move |action| match executor.execute(&action) {
                    Ok(_) => {}
                    Err(e) => error!("Cannot log action: {}", e),
                }),
            );

            Arc::new(event_bus)
        };

        // Start executor
        let executor_actor = SyncArbiter::start(1, move || {
            let dispatcher = Dispatcher::new(event_bus.clone()).unwrap();
            ExecutorActor { dispatcher }
        });

        // Start engine
        let matcher_actor = SyncArbiter::start(cpus, move || MatcherActor {
            matcher: matcher.clone(),
            executor_addr: executor_actor.clone(),
        });

        // Start collector
        listen_to_uds_socket(&conf.io.uds_socket_path, move |msg| {
            JsonReaderActor::start_new(msg, matcher_actor.clone());
        });
    });
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
