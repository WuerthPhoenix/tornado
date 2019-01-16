pub mod collector;
pub mod config;
pub mod dispatcher;
pub mod engine;
pub mod executor;
pub mod io;

use crate::dispatcher::{ActixEventBus, DispatcherActor};
use crate::engine::MatcherActor;
use crate::executor::ActionMessage;
use crate::executor::ExecutorActor;
use crate::io::uds::listen_to_uds_socket;
use actix::prelude::*;
use log::*;
use std::fs;
use std::sync::Arc;
use tornado_common_logger::setup_logger;
use tornado_engine_matcher::config::Rule;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tornado_engine_matcher::matcher::Matcher;

fn main() {
    let conf = config::Conf::build();

    setup_logger(&conf.logger).expect("Cannot configure the logger");

    // Load rules from fs
    let config_rules =
        read_rules_from_config(&format!("{}/{}", conf.io.config_dir, conf.io.rules_dir));

    // Start matcher
    let matcher = Arc::new(
        Matcher::build(&config_rules).unwrap_or_else(|err| panic!("Cannot parse rules: {}", err)),
    );

    // start system
    System::run(move || {
        let cpus = num_cpus::get();
        info!("Available CPUs: {}", cpus);

        // Start archive executor actor
        let archive_config_file_path = format!("{}/archive_executor.toml", conf.io.config_dir);
        let archive_executor_addr = SyncArbiter::start(1, move || {
            let archive_config = config::build_archive_config(&archive_config_file_path.clone())
                .expect("Cannot build the ArchiveExecutor configuration");

            let executor = tornado_executor_archive::ArchiveExecutor::new(&archive_config);
            ExecutorActor { executor }
        });

        // Start script executor actor
        let script_executor_addr = SyncArbiter::start(1, move || {
            let executor = tornado_executor_script::ScriptExecutor::new();
            ExecutorActor { executor }
        });

        // Configure action dispatcher
        let event_bus = {
            let event_bus = ActixEventBus {
                callback: move |action| {
                    match action.id.as_ref() {
                        "archive" => archive_executor_addr.do_send(ActionMessage { action }),
                        "script" => script_executor_addr.do_send(ActionMessage { action }),
                        _ => error!("There are not executors for action id [{}]", &action.id),
                    };
                },
            };
            Arc::new(event_bus)
        };

        // Start dispatcher actor
        let dispatcher_addr = SyncArbiter::start(1, move || {
            let dispatcher =
                Dispatcher::build(event_bus.clone()).expect("Cannot build the dispatcher");
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
            collector::event::EventJsonReaderActor::start_new(msg, json_matcher_addr_clone.clone());
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
        let filename = path.expect("Cannot get the filename").path();
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
