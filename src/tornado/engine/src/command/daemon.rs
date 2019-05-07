use crate::config;
use crate::dispatcher::{ActixEventBus, DispatcherActor};
use crate::engine::{EventMessage, MatcherActor};
use crate::executor::icinga2::{Icinga2ApiClientActor, Icinga2ApiClientMessage};
use crate::executor::ActionMessage;
use crate::executor::ExecutorActor;

use crate::monitoring::monitoring_endpoints;
use actix::prelude::*;
use actix_web::{web, App, HttpServer};
use failure::Fail;
use log::*;
use std::sync::Arc;
use tornado_common::actors::json_event_reader::JsonEventReaderActor;
use tornado_common::actors::tcp_server::listen_to_tcp;
use tornado_common_logger::setup_logger;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tornado_engine_matcher::matcher::Matcher;

pub fn daemon(
    conf: &config::Conf,
    daemon_config: config::DaemonCommandConfig,
) -> Result<(), Box<std::error::Error>> {
    setup_logger(&conf.logger).map_err(Fail::compat)?;

    let configs = config::parse_config_files(conf)?;

    // Start matcher
    let matcher = Arc::new(
        configs
            .matcher_config
            .read()
            .and_then(|config| Matcher::build(&config))
            .map_err(Fail::compat)?,
    );

    // start system
    System::run(move || {
        let cpus = num_cpus::get();
        info!("Available CPUs: {}", cpus);

        // Start archive executor actor
        let archive_config = configs.archive.clone();
        let archive_executor_addr = SyncArbiter::start(1, move || {
            let executor = tornado_executor_archive::ArchiveExecutor::new(&archive_config);
            ExecutorActor { executor }
        });

        // Start script executor actor
        let script_executor_addr = SyncArbiter::start(1, move || {
            let executor = tornado_executor_script::ScriptExecutor::new();
            ExecutorActor { executor }
        });

        // Start Icinga2 Client Actor
        let icinga2_client_addr = Icinga2ApiClientActor::start_new(configs.icinga2_client);

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

        // Start Event Json TCP listener
        let tcp_address =
            format!("{}:{}", daemon_config.event_socket_ip, daemon_config.event_socket_port);
        let json_matcher_addr_clone = matcher_addr.clone();
        listen_to_tcp(tcp_address.clone(), move |msg| {
            let json_matcher_addr_clone = json_matcher_addr_clone.clone();
            JsonEventReaderActor::start_new(msg, move |event| {
                json_matcher_addr_clone.do_send(EventMessage { event })
            });
        })
        .and_then(|_| {
            info!("Started TCP server at [{}]. Listening for incoming events", tcp_address);
            Ok(())
        })
        // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
        .unwrap_or_else(|err| {
            error!("Cannot start TCP server at [{}]. Err: {}", tcp_address, err);
            std::process::exit(1);
        });

        let web_server_ip = daemon_config.web_server_ip.clone();
        let web_server_port = daemon_config.web_server_port;
        let matcher_config = configs.matcher_config;

        let api_handler =
            Arc::new(backend::api::matcher::MatcherApiHandler { config_manager: matcher_config });

        // Start API and monitoring endpoint
        HttpServer::new(move || {
            App::new()
                .service(monitoring_endpoints(web::scope("/monitoring")))
                .service(backend::api::new_endpoints(web::scope("/api"), api_handler.clone()))
        })
        .bind(format!("{}:{}", web_server_ip, web_server_port))
        // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
        .unwrap_or_else(|err| {
            error!("Web Server cannot start on port {}. Err: {}", web_server_port, err);
            //System::current().stop_with_code(1);
            std::process::exit(1);
        })
        .start();
    })?;

    Ok(())
}
