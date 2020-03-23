use crate::api::MatcherApiHandler;
use crate::config;
use crate::dispatcher::{ActixEventBus, DispatcherActor};
use crate::engine::{EventMessage, MatcherActor};
use crate::executor::icinga2::{Icinga2ApiClientActor, Icinga2ApiClientMessage};
use crate::executor::ExecutorActor;
use crate::executor::{ActionMessage, LazyExecutorActor, LazyExecutorActorInitMessage};
use crate::monitoring::monitoring_endpoints;
use actix::prelude::*;
use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use log::*;
use std::sync::Arc;
use tornado_common::actors::json_event_reader::JsonEventReaderActor;
use tornado_common::actors::nats_subscriber::subscribe_to_nats;
use tornado_common::actors::tcp_server::listen_to_tcp;
use tornado_common_logger::setup_logger;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tornado_engine_matcher::matcher::Matcher;

pub async fn daemon(
    config_dir: &str,
    rules_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let configs = config::parse_config_files(config_dir, rules_dir)?;

    setup_logger(&configs.tornado.logger)?;

    // Start matcher
    let matcher =
        Arc::new(configs.matcher_config.read().and_then(|config| Matcher::build(&config))?);

    // start system

    let cpus = num_cpus::get();
    debug!("Available CPUs: {}", cpus);

    let daemon_config = configs.tornado.tornado.daemon;

    // Start archive executor actor
    let archive_config = configs.archive_executor_config.clone();
    let archive_executor_addr = SyncArbiter::start(1, move || {
        let executor = tornado_executor_archive::ArchiveExecutor::new(&archive_config);
        ExecutorActor { executor }
    });

    // Start script executor actor
    let script_executor_addr = SyncArbiter::start(1, move || {
        let executor = tornado_executor_script::ScriptExecutor::new();
        ExecutorActor { executor }
    });

    // Start logger executor actor
    let logger_executor_addr = SyncArbiter::start(1, move || {
        let executor = tornado_executor_logger::LoggerExecutor::new();
        ExecutorActor { executor }
    });

    // Start Icinga2 Client Actor
    let icinga2_client_addr = Icinga2ApiClientActor::start_new(configs.icinga2_executor_config);

    // Start ForEach executor actor
    let foreach_executor_addr = SyncArbiter::start(1, move || LazyExecutorActor::<
        tornado_executor_foreach::ForEachExecutor,
    > {
        executor: None,
    });

    // Start elasticsearch executor actor
    let es_authentication = configs.elasticsearch_executor_config.default_auth.clone();
    let elasticsearch_executor_addr = SyncArbiter::start(1, move || {
        let es_authentication = es_authentication.clone();
        let executor =
            tornado_executor_elasticsearch::ElasticsearchExecutor::new(es_authentication)
                .expect("Cannot start the Elasticsearch Executor");
        ExecutorActor { executor }
    });

    // Start icinga2 executor actor
    let icinga2_executor_addr = SyncArbiter::start(1, move || {
        let icinga2_client_addr_clone = icinga2_client_addr.clone();
        let executor = tornado_executor_icinga2::Icinga2Executor::new(move |icinga2action| {
            icinga2_client_addr_clone.do_send(Icinga2ApiClientMessage { message: icinga2action });
            Ok(())
        });
        ExecutorActor { executor }
    });

    // Configure action dispatcher
    let foreach_executor_addr_clone = foreach_executor_addr.clone();
    let event_bus = {
        let event_bus = ActixEventBus {
            callback: move |action| {
                match action.id.as_ref() {
                    "archive" => archive_executor_addr.do_send(ActionMessage { action }),
                    "icinga2" => icinga2_executor_addr.do_send(ActionMessage { action }),
                    "script" => script_executor_addr.do_send(ActionMessage { action }),
                    "foreach" => foreach_executor_addr_clone.do_send(ActionMessage { action }),
                    "logger" => logger_executor_addr.do_send(ActionMessage { action }),
                    "elasticsearch" => {
                        elasticsearch_executor_addr.do_send(ActionMessage { action })
                    }
                    _ => error!("There are not executors for action id [{}]", &action.id),
                };
            },
        };
        Arc::new(event_bus)
    };

    let event_bus_clone = event_bus.clone();
    foreach_executor_addr.do_send(LazyExecutorActorInitMessage::<
        tornado_executor_foreach::ForEachExecutor,
        _,
    > {
        init: move || tornado_executor_foreach::ForEachExecutor::new(event_bus_clone.clone()),
    });

    // Start dispatcher actor
    let dispatcher_addr = SyncArbiter::start(1, move || {
        let dispatcher = Dispatcher::build(event_bus.clone()).expect("Cannot build the dispatcher");
        DispatcherActor { dispatcher }
    });

    // Start matcher actor
    let matcher_addr = SyncArbiter::start(cpus, move || MatcherActor {
        matcher: matcher.clone(),
        dispatcher_addr: dispatcher_addr.clone(),
    });

    if daemon_config.get_nats_streaming_enabled() {
        info!("NATS Streaming connection is enabled. Starting it...");

        let nats_config = daemon_config
            .nats
            .clone()
            .expect("Nats configuration must be provided to connect to the Nats cluster");

        let addresses = nats_config.client.addresses.clone();
        let subject = nats_config.client.subject.clone();

        let matcher_addr_clone = matcher_addr.clone();
        subscribe_to_nats(nats_config, daemon_config.message_queue_size, move |event| {
            matcher_addr_clone.do_send(EventMessage { event });
            Ok(())
        }).await
            .and_then(|_| {
                info!("NATS Streaming connection started at [{:#?}]. Listening for incoming events on subject [{}]", addresses, subject);
                Ok(())
            })
            .unwrap_or_else(|err| {
                error!("NATS Streaming connection failed started at [{:#?}], subject [{}]. Err: {}", addresses, subject, err);
                std::process::exit(1);
            });
    } else {
        info!("NATS Streaming connection is disabled. Do not start it.")
    };

    if daemon_config.get_event_tcp_socket_enabled() {
        info!("TCP server is enabled. Starting it...");
        // Start Event Json TCP listener
        let tcp_address = format!(
            "{}:{}",
            daemon_config
                .event_socket_ip
                .clone()
                .expect("'event_socket_ip' must be provided to start the tornado TCP server"),
            daemon_config
                .event_socket_port
                .clone()
                .expect("'event_socket_port' must be provided to start the tornado TCP server")
        );
        let json_matcher_addr_clone = matcher_addr.clone();
        listen_to_tcp(tcp_address.clone(), daemon_config.message_queue_size, move |msg| {
            let json_matcher_addr_clone = json_matcher_addr_clone.clone();
            JsonEventReaderActor::start_new(msg, move |event| {
                json_matcher_addr_clone.do_send(EventMessage { event })
            });
        })
        .await
        .and_then(|_| {
            info!("Started TCP server at [{}]. Listening for incoming events", tcp_address);
            Ok(())
        })
        // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
        .unwrap_or_else(|err| {
            error!("Cannot start TCP server at [{}]. Err: {}", tcp_address, err);
            std::process::exit(1);
        });
    } else {
        info!("TCP server is disabled. Do not start it.")
    };

    let web_server_ip = daemon_config.web_server_ip.clone();
    let web_server_port = daemon_config.web_server_port;
    let matcher_config = configs.matcher_config;

    let api_handler = MatcherApiHandler::new(matcher_config, matcher_addr);
    let daemon_config = daemon_config.clone();

    // Start API and monitoring endpoint
    HttpServer::new(move || {
        let api_handler = api_handler.clone();
        let daemon_config = daemon_config.clone();
        App::new()
            .wrap(Cors::new().max_age(3600).finish())
            .service({ tornado_engine_api::api::new_endpoints(web::scope("/api"), api_handler) })
            .service(monitoring_endpoints(web::scope("/monitoring"), daemon_config))
    })
    .bind(format!("{}:{}", web_server_ip, web_server_port))
    // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
    .unwrap_or_else(|err| {
        error!("Web Server cannot start on port {}. Err: {}", web_server_port, err);
        std::process::exit(1);
    })
    .run()
    .await?;

    Ok(())
}
