pub mod collector;
pub mod config;
pub mod dispatcher;
pub mod engine;
pub mod executor;

use crate::dispatcher::{ActixEventBus, DispatcherActor};
use crate::engine::MatcherActor;
use crate::executor::icinga2::Icinga2ApiClientMessage;
use crate::executor::ActionMessage;
use crate::executor::ExecutorActor;
use actix::prelude::*;
use failure::Fail;
use log::*;
use std::sync::Arc;
use tornado_common::actors::uds_reader::listen_to_uds_socket;
use tornado_common_logger::setup_logger;
use tornado_engine_matcher::config::Config;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tornado_engine_matcher::matcher::Matcher;

fn main() -> Result<(), Box<std::error::Error>> {
    let conf = config::Conf::build();

    setup_logger(&conf.logger).map_err(|e| e.compat())?;

    // Load rules from fs
    let config_rules = Config::read_rules_from_dir_sorted_by_filename(&format!(
        "{}/{}",
        conf.io.config_dir, conf.io.rules_dir
    ))
    .map_err(|e| e.compat())?;

    // Start matcher
    let matcher = Arc::new(Matcher::build(&config_rules).map_err(|e| e.compat())?);

    // start system
    System::run(move || {
        let cpus = num_cpus::get();
        info!("Available CPUs: {}", cpus);

        // Start archive executor actor
        let archive_config_file_path = format!("{}/archive_executor.toml", conf.io.config_dir);
        let archive_executor_addr = SyncArbiter::start(1, move || {
            let archive_config = config::build_archive_config(&archive_config_file_path.clone())
                .unwrap_or_else(|err| {
                    error!("Cannot build the ArchiveExecutor configuration. Err: {}", err);
                    std::process::exit(1);
                });

            let executor = tornado_executor_archive::ArchiveExecutor::new(&archive_config);
            ExecutorActor { executor }
        });

        // Start script executor actor
        let script_executor_addr = SyncArbiter::start(1, move || {
            let executor = tornado_executor_script::ScriptExecutor::new();
            ExecutorActor { executor }
        });

        // Start Icinga2 Client Actor
        let icinga2_client_config_file_path =
            format!("{}/icinga2_client_executor.toml", conf.io.config_dir);
        let icinga2_client_config = config::build_icinga2_client_config(
            &icinga2_client_config_file_path,
        )
        .unwrap_or_else(|err| {
            error!("Cannot build the Icinga2ApiClientActor configuration. Err: {}", err);
            std::process::exit(1);
        });
        let icinga2_client_addr =
            executor::icinga2::Icinga2ApiClientActor::start_new(icinga2_client_config);

        // Start icinga2 executor actor
        let icinga2_executor_addr = SyncArbiter::start(1, move || {
            let icinga2_client_addr_clone = icinga2_client_addr.clone();
            let executor = tornado_executor_icinga2::Icinga2Executor::new(move |icinga2action| {
                icinga2_client_addr_clone
                    .do_send(Icinga2ApiClientMessage { message: icinga2action });
                Ok(())
            });
            ExecutorActor { executor }
        });

        // Configure action dispatcher
        let event_bus = {
            let event_bus = ActixEventBus {
                callback: move |action| {
                    match action.id.as_ref() {
                        "archive" => archive_executor_addr.do_send(ActionMessage { action }),
                        "icinga2" => icinga2_executor_addr.do_send(ActionMessage { action }),
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
        listen_to_uds_socket(conf.io.uds_path.clone(), move |msg| {
            collector::event::EventJsonReaderActor::start_new(msg, json_matcher_addr_clone.clone());
        })
        // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
        .unwrap_or_else(|err| {
            error!("Cannot start uds socket reader on path [{}]. Err: {}", conf.io.uds_path, err);
            std::process::exit(1);
        });

        // Start snmptrapd Json UDS listener
        let snmptrapd_matcher_addr_clone = matcher_addr.clone();
        listen_to_uds_socket(conf.io.snmptrapd_uds_path.clone(), move |msg| {
            collector::snmptrapd::SnmptrapdJsonReaderActor::start_new(
                msg,
                snmptrapd_matcher_addr_clone.clone(),
            );
        })
        // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
        .unwrap_or_else(|err| {
            error!(
                "Cannot start uds socket reader on path [{}]. Err: {}",
                conf.io.snmptrapd_uds_path, err
            );
            std::process::exit(1);
        });
    });

    Ok(())
}
