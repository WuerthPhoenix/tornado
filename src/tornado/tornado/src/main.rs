extern crate tornado_collector_common;
extern crate tornado_collector_json;
extern crate tornado_collector_snmptrapd;
extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_engine_matcher;
extern crate tornado_executor_archive;
extern crate tornado_executor_common;
extern crate tornado_network_common;
extern crate tornado_network_simple;

extern crate actix;
extern crate futures;
#[macro_use]
extern crate log;
extern crate num_cpus;
extern crate serde;
#[macro_use]
extern crate structopt;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_uds;

pub mod collector;
pub mod config;
pub mod dispatcher;
pub mod engine;
pub mod executor;
pub mod io;

use actix::prelude::*;
use engine::MatcherActor;
use dispatcher::DispatcherActor;
use io::uds::listen_to_uds_socket;
use std::fs;
use std::sync::Arc;
use tornado_common_logger::setup_logger;
use tornado_engine_matcher::config::Rule;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tornado_engine_matcher::matcher::Matcher;
use tornado_executor_common::Executor;
use tornado_network_simple::SimpleEventBus;
use std::collections::HashMap;

fn main() {
    let conf = config::Conf::build();

    setup_logger(&conf.logger).unwrap();

    // Load rules from fs
    let config_rules = read_rules_from_config(&conf.io.rules_dir);

    // Start matcher
    let matcher = Arc::new(
        Matcher::new(&config_rules).unwrap_or_else(|err| panic!("Cannot parse rules: {}", err)),
    );

    // start system
    System::run(move || {
        let cpus = num_cpus::get();
        info!("Available CPUs: {}", cpus);

        // Configure action dispatcher
        let event_bus = {
            let mut event_bus = SimpleEventBus::new();

            let archive_config = tornado_executor_archive::config::ArchiveConfig {
                base_path: "./target".to_owned(),
                default_path: "/default/file.out".to_owned(),
                paths: HashMap::new(),
                file_cache_size: 10,
                file_cache_ttl_secs: 1,
            };

            // ToDo the eventbus needs to be reworked when the archive executor is completed
            let mut archive_executor = tornado_executor_archive::ArchiveExecutor::new(&archive_config);
            event_bus.subscribe_to_action(
                "archive",
                Box::new(move |action| {
                    /*
                    match archive_executor.execute(&action) {
                        Ok(_) => {}
                        Err(e) => error!("Cannot log action: {}", e),
                    }
                    */
                }),
            );

            Arc::new(event_bus)
        };

        // Start executor actor
        let dispatcher_addr = SyncArbiter::start(1, move || {
            let dispatcher = Dispatcher::new(event_bus.clone()).unwrap();
            DispatcherActor { dispatcher }
        });

        // Start matcher actor
        let matcher_addr = SyncArbiter::start(cpus, move || MatcherActor {
            matcher: matcher.clone(),
            dispatcher_addr: dispatcher_addr.clone(),
        });

        // Start Event Json UDS listener
        let json_matcher_addr_clone = matcher_addr.clone();
        listen_to_uds_socket(conf.io.uds_path, move |msg| {
            collector::event::EventJsonReaderActor::start_new(
                msg,
                json_matcher_addr_clone.clone(),
            );
        });

        // Start snmptrapd Json UDS listener
        let snmptrapd_matcher_addr_clone = matcher_addr.clone();
        listen_to_uds_socket(conf.io.snmptrapd_uds_path, move |msg| {
            collector::snmptrapd::SnmptrapdJsonReaderActor::start_new(
                msg,
                snmptrapd_matcher_addr_clone.clone(),
            );
        });
    });
}

fn read_rules_from_config(path: &str) -> Vec<Rule> {
    let paths = fs::read_dir(path)
        .unwrap_or_else(|err| panic!("Cannot access specified folder [{}]: {}", path, err));
    let mut rules = vec![];

    for path in paths {
        let filename = path.unwrap().path();
        info!("Loading rule from file: [{}]", filename.display());
        let rule_body = fs::read_to_string(&filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename.display()));
        trace!("Rule body: \n{}", rule_body);
        rules.push(Rule::from_json(&rule_body).unwrap_or_else(|err| {
            panic!("Cannot build rule from provided: [{:?}] \n error: [{}]", &rule_body, err)
        }));
    }

    info!("Loaded {} rule(s) from [{}]", rules.len(), path);

    rules
}
