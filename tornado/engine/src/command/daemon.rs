use crate::api::MatcherApiHandler;
use crate::config;
use crate::dispatcher::{ActixEventBus, DispatcherActor};
use crate::engine::{EventMessage, MatcherActor};
use crate::executor::director::DirectorApiClientMessage;
use crate::executor::icinga2::Icinga2ApiClientMessage;
use crate::executor::ApiClientActor;
use crate::executor::ExecutorActor;
use crate::executor::{ActionMessage, LazyExecutorActor, LazyExecutorActorInitMessage};
use crate::monitoring::monitoring_endpoints;
use actix::prelude::*;
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use log::*;
use std::sync::Arc;
use tornado_common::actors::json_event_reader::JsonEventReaderActor;
use tornado_common::actors::nats_subscriber::subscribe_to_nats;
use tornado_common::actors::tcp_server::listen_to_tcp;
use tornado_common_logger::setup_logger;
use tornado_engine_api::auth::{roles_map_to_permissions_map, AuthService};
use tornado_engine_api::config::api::ConfigApi;
use tornado_engine_api::model::ApiData;
use tornado_engine_matcher::dispatcher::Dispatcher;

pub async fn daemon(
    config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let configs = config::parse_config_files(config_dir, rules_dir, drafts_dir)?;

    setup_logger(&configs.tornado.logger)?;

    // start system
    let daemon_config = configs.tornado.tornado.daemon;
    let thread_pool_config = daemon_config.thread_pool_config.clone().unwrap_or_default();
    let threads_per_queue = thread_pool_config.get_threads_count();
    info!(
        "Starting Tornado daemon with {} threads per queue. Thread config: {:?}",
        threads_per_queue, thread_pool_config
    );

    // Start archive executor actor
    let archive_config = configs.archive_executor_config.clone();
    let archive_executor_addr = SyncArbiter::start(threads_per_queue, move || {
        let executor = tornado_executor_archive::ArchiveExecutor::new(&archive_config);
        ExecutorActor { executor }
    });

    // Start script executor actor
    let script_executor_addr = SyncArbiter::start(threads_per_queue, move || {
        let executor = tornado_executor_script::ScriptExecutor::new();
        ExecutorActor { executor }
    });

    // Start logger executor actor
    let logger_executor_addr = SyncArbiter::start(threads_per_queue, move || {
        let executor = tornado_executor_logger::LoggerExecutor::new();
        ExecutorActor { executor }
    });

    // Start Api Client Actor for Icinga2
    let icinga2_client_addr = ApiClientActor::start_new(configs.icinga2_executor_config);

    // Start Api Client Actor for Director
    let director_client_addr = ApiClientActor::start_new(configs.director_executor_config);

    // Start ForEach executor actor
    let foreach_executor_addr = SyncArbiter::start(threads_per_queue, move || LazyExecutorActor::<
        tornado_executor_foreach::ForEachExecutor,
    > {
        executor: None,
    });

    // Start elasticsearch executor actor
    let es_authentication = configs.elasticsearch_executor_config.default_auth.clone();
    let elasticsearch_executor_addr = SyncArbiter::start(threads_per_queue, move || {
        let es_authentication = es_authentication.clone();
        let executor =
            tornado_executor_elasticsearch::ElasticsearchExecutor::new(es_authentication)
                .expect("Cannot start the Elasticsearch Executor");
        ExecutorActor { executor }
    });

    // Start icinga2 executor actor
    let icinga2_executor_addr = SyncArbiter::start(threads_per_queue, move || {
        let icinga2_client_addr_clone = icinga2_client_addr.clone();
        let executor = tornado_executor_icinga2::Icinga2Executor::new(move |icinga2action| {
            icinga2_client_addr_clone.do_send(Icinga2ApiClientMessage { message: icinga2action });
            Ok(())
        });
        ExecutorActor { executor }
    });

    // Start director executor actor
    let director_executor_addr = SyncArbiter::start(threads_per_queue, move || {
        let director_client_addr_clone = director_client_addr.clone();
        let executor = tornado_executor_director::DirectorExecutor::new(move |director_action| {
            director_client_addr_clone
                .do_send(DirectorApiClientMessage { message: director_action });
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
                    "director" => director_executor_addr.do_send(ActionMessage { action }),
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
    let dispatcher_addr = SyncArbiter::start(threads_per_queue, move || {
        let dispatcher = Dispatcher::build(event_bus.clone()).expect("Cannot build the dispatcher");
        DispatcherActor { dispatcher }
    });

    // Start matcher actor
    let matcher_addr =
        MatcherActor::start(dispatcher_addr.clone(), configs.matcher_config.clone())?;

    if daemon_config.is_nats_enabled() {
        info!("NATS connection is enabled. Starting it...");

        let nats_config = daemon_config
            .nats
            .clone()
            .expect("Nats configuration must be provided to connect to the Nats cluster");

        let addresses = nats_config.client.addresses.clone();
        let subject = nats_config.subject.clone();
        let message_queue_size = daemon_config.message_queue_size;
        let matcher_addr_clone = matcher_addr.clone();

        actix::spawn(async move {
            subscribe_to_nats(nats_config, message_queue_size, move |event| {
                matcher_addr_clone.do_send(EventMessage { event });
                Ok(())
            })
            .await
            .and_then(|_| {
                info!(
                    "NATS connection started at [{:#?}]. Listening for incoming events on subject [{}]",
                    addresses, subject
                );
                Ok(())
            })
            .unwrap_or_else(|err| {
                error!(
                    "NATS connection failed started at [{:#?}], subject [{}]. Err: {}",
                    addresses, subject, err
                );
                std::process::exit(1);
            });
        });
    } else {
        info!("NATS connection is disabled. Do not start it.")
    };

    if daemon_config.is_event_tcp_socket_enabled() {
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
        let message_queue_size = daemon_config.message_queue_size;

        actix::spawn(async move {
            listen_to_tcp(tcp_address.clone(), message_queue_size, move |msg| {
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
        });
    } else {
        info!("TCP server is disabled. Do not start it.")
    };

    let web_server_ip = daemon_config.web_server_ip.clone();
    let web_server_port = daemon_config.web_server_port;

    let auth_service = AuthService::new(Arc::new(roles_map_to_permissions_map(
        daemon_config.auth.role_permissions.clone(),
    )));
    let api_handler = MatcherApiHandler::new(matcher_addr);
    let daemon_config = daemon_config.clone();
    let matcher_config = configs.matcher_config.clone();

    // Start API and monitoring endpoint
    HttpServer::new(move || {
        let api_handler = api_handler.clone();
        let daemon_config = daemon_config.clone();
        let config_api = ApiData {
            auth: auth_service.clone(),
            api: ConfigApi::new(api_handler.clone(), matcher_config.clone()),
        };
        let auth_api = ApiData { auth: auth_service.clone(), api: () };

        App::new()
            .wrap(Logger::default())
            .wrap(Cors::new().max_age(3600).finish())
            .service(
                web::scope("/api")
                    .service(tornado_engine_api::auth::web::build_auth_endpoints(auth_api))
                    .service(tornado_engine_api::config::web::build_config_endpoints(config_api))
                    .service(tornado_engine_api::event::web::build_event_endpoints(api_handler)),
            )
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
