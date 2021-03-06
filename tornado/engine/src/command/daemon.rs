use crate::api::MatcherApiHandler;
use crate::config;
use crate::config::build_config;
use crate::dispatcher::{ActixEventBus, DispatcherActor};
use crate::engine::{EventMessage, MatcherActor};
use crate::executor::foreach::{ForEachExecutorActor, ForEachExecutorActorInitMessage};
use crate::executor::retry::RetryActor;
use crate::executor::ActionMessage;
use crate::executor::ExecutorRunner;
use crate::monitoring::monitoring_endpoints;
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use log::*;
use std::sync::Arc;
use tornado_common::actors::json_event_reader::JsonEventReaderActor;
use tornado_common::actors::message::TornadoCommonActorError;
use tornado_common::actors::nats_subscriber::subscribe_to_nats;
use tornado_common::actors::tcp_server::listen_to_tcp;
use tornado_common::pool::blocking_pool::start_blocking_runner;
use tornado_common_logger::setup_logger;
use tornado_engine_api::auth::{roles_map_to_permissions_map, AuthService};
use tornado_engine_api::config::api::ConfigApi;
use tornado_engine_api::event::api::EventApi;
use tornado_engine_api::model::ApiData;
use tornado_engine_matcher::dispatcher::Dispatcher;

pub const ACTION_ID_SMART_MONITORING_CHECK_RESULT: &str = "smart_monitoring_check_result";
pub const ACTION_ID_MONITORING: &str = "monitoring";
pub const ACTION_ID_FOREACH: &str = "foreach";
pub const ACTION_ID_LOGGER: &str = "logger";

// 64*1024*1024 byte = 64MB limit
const MAX_JSON_PAYLOAD_SIZE: usize = 67_108_860;

pub async fn daemon(
    config_dir: &str,
    rules_dir: &str,
    drafts_dir: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let tornado = build_config(config_dir)?;
    let _guard = setup_logger(&tornado.logger)?;

    let configs = config::parse_config_files(config_dir, rules_dir, drafts_dir)?;

    // start system
    let daemon_config = tornado.tornado.daemon;
    let thread_pool_config = daemon_config.thread_pool_config.clone().unwrap_or_default();
    let threads_per_queue = thread_pool_config.get_threads_count();
    info!(
        "Starting Tornado daemon with {} threads per queue. Thread config: {:?}",
        threads_per_queue, thread_pool_config
    );

    let retry_strategy = Arc::new(daemon_config.retry_strategy.clone());
    info!("Tornado global retry strategy: {:?}", retry_strategy);

    let message_queue_size = daemon_config.message_queue_size;

    // Start archive executor actor
    let archive_config = configs.archive_executor_config.clone();
    let archive_executor_addr =
        RetryActor::start_new(message_queue_size, retry_strategy.clone(), move || {
            start_blocking_runner(threads_per_queue, message_queue_size, || {
                let executor = tornado_executor_archive::ArchiveExecutor::new(&archive_config);
                ExecutorRunner { executor }
            })
        });

    // Start script executor actor
    let script_executor_addr = {
        let executor = tornado_executor_script::ScriptExecutor::new();
        RetryActor::start_new(message_queue_size, retry_strategy.clone(), move || {
            start_blocking_runner(threads_per_queue, message_queue_size, || ExecutorRunner {
                executor: executor.clone(),
            })
        })
    };

    // Start logger executor actor
    let logger_executor_addr = {
        let executor = tornado_executor_logger::LoggerExecutor::new();
        RetryActor::start_new(message_queue_size, retry_strategy.clone(), move || {
            start_blocking_runner(threads_per_queue, message_queue_size, || ExecutorRunner {
                executor: executor.clone(),
            })
        })
    };

    // Start ForEach executor actor
    let foreach_executor_addr = ForEachExecutorActor::start_new(message_queue_size);

    // Start elasticsearch executor actor
    let elasticsearch_executor_addr = {
        let es_authentication = configs.elasticsearch_executor_config.default_auth.clone();
        let executor =
            tornado_executor_elasticsearch::ElasticsearchExecutor::new(es_authentication)
                .expect("Cannot start the Elasticsearch Executor");
        RetryActor::start_new(message_queue_size, retry_strategy.clone(), move || {
            start_blocking_runner(threads_per_queue, message_queue_size, || ExecutorRunner {
                executor: executor.clone(),
            })
        })
    };

    // Start icinga2 executor actor
    let icinga2_executor_addr = {
        let executor =
            tornado_executor_icinga2::Icinga2Executor::new(configs.icinga2_executor_config.clone())
                .expect("Cannot start the Icinga2Executor Executor");
        RetryActor::start_new(message_queue_size, retry_strategy.clone(), move || {
            start_blocking_runner(threads_per_queue, message_queue_size, || ExecutorRunner {
                executor: executor.clone(),
            })
        })
    };

    // Start director executor actor
    let director_client_config = configs.director_executor_config.clone();
    let director_executor_addr = {
        let executor =
            tornado_executor_director::DirectorExecutor::new(director_client_config.clone())
                .expect("Cannot start the DirectorExecutor Executor");
        RetryActor::start_new(message_queue_size, retry_strategy.clone(), move || {
            start_blocking_runner(threads_per_queue, message_queue_size, || ExecutorRunner {
                executor: executor.clone(),
            })
        })
    };

    // Start monitoring executor actor
    let monitoring_executor_addr = {
        let executor = tornado_executor_monitoring::MonitoringExecutor::new(
            configs.icinga2_executor_config.clone(),
            configs.director_executor_config.clone(),
        )
        .expect("Cannot start the MonitoringExecutor Executor");
        RetryActor::start_new(message_queue_size, retry_strategy.clone(), move || {
            start_blocking_runner(threads_per_queue, message_queue_size, || ExecutorRunner {
                executor: executor.clone(),
            })
        })
    };

    // Start smart_monitoring_check_result executor actor
    let smart_monitoring_check_result_executor_addr = {
        let executor =
            tornado_executor_smart_monitoring_check_result::SmartMonitoringExecutor::new(
                configs.smart_monitoring_check_result_config.clone(),
                configs.icinga2_executor_config.clone(),
                configs.director_executor_config.clone(),
            )
            .expect("Cannot start the SmartMonitoringExecutor Executor");
        RetryActor::start_new(message_queue_size, retry_strategy.clone(), move || {
            start_blocking_runner(threads_per_queue, message_queue_size, || ExecutorRunner {
                executor: executor.clone(),
            })
        })
    };

    // Configure action dispatcher
    let foreach_executor_addr_clone = foreach_executor_addr.clone();
    let event_bus = {
        let event_bus = ActixEventBus {
            callback: move |action| {
                let action = Arc::new(action);
                let send_result = match action.id.as_ref() {
                    "archive" => {
                        archive_executor_addr.try_send(ActionMessage { action }).map_err(|err| {
                            format!("Error sending message to 'archive' executor. Err: {:?}", err)
                        })
                    }
                    "icinga2" => {
                        icinga2_executor_addr.try_send(ActionMessage { action }).map_err(|err| {
                            format!("Error sending message to 'icinga2' executor. Err: {:?}", err)
                        })
                    }
                    "director" => {
                        director_executor_addr.try_send(ActionMessage { action }).map_err(|err| {
                            format!("Error sending message to 'director' executor. Err: {:?}", err)
                        })
                    }
                    ACTION_ID_MONITORING => {
                        monitoring_executor_addr.try_send(ActionMessage { action }).map_err(|err| {
                            format!(
                                "Error sending message to 'monitoring' executor. Err: {:?}",
                                err
                            )
                        })
                    }
                    ACTION_ID_SMART_MONITORING_CHECK_RESULT => {
                        smart_monitoring_check_result_executor_addr.try_send(ActionMessage { action }).map_err(|err| {
                            format!(
                                "Error sending message to 'smart_monitoring_check_result' executor. Err: {:?}",
                                err
                            )
                        })
                    }
                    "script" => {
                        script_executor_addr.try_send(ActionMessage { action }).map_err(|err| {
                            format!("Error sending message to 'script' executor. Err: {:?}", err)
                        })
                    }
                    ACTION_ID_FOREACH => foreach_executor_addr_clone
                        .try_send(ActionMessage { action })
                        .map_err(|err| {
                            format!("Error sending message to 'foreach' executor. Err: {:?}", err)
                        }),
                    ACTION_ID_LOGGER => {
                        logger_executor_addr.try_send(ActionMessage { action }).map_err(|err| {
                            format!("Error sending message to 'logger' executor. Err: {:?}", err)
                        })
                    }
                    "elasticsearch" => elasticsearch_executor_addr
                        .try_send(ActionMessage { action })
                        .map_err(|err| {
                            format!(
                                "Error sending message to 'elasticsearch' executor. Err: {:?}",
                                err
                            )
                        }),
                    _ => Err(format!("There are not executors for action id [{}]", &action.id)),
                };
                if let Err(error_message) = send_result {
                    error!("{}", error_message)
                }
            },
        };
        Arc::new(event_bus)
    };

    let event_bus_clone = event_bus.clone();
    foreach_executor_addr.try_send(ForEachExecutorActorInitMessage {
        init: move || tornado_executor_foreach::ForEachExecutor::new(event_bus_clone.clone()),
    })?;

    // Start dispatcher actor
    let dispatcher_addr = DispatcherActor::start_new(
        message_queue_size,
        Dispatcher::build(event_bus.clone()).expect("Cannot build the dispatcher"),
    );

    // Start matcher actor
    let matcher_addr = MatcherActor::start(
        dispatcher_addr.clone(),
        configs.matcher_config.clone(),
        message_queue_size,
    )?;

    if daemon_config.is_nats_enabled() {
        info!("NATS connection is enabled. Starting it...");

        let nats_config = daemon_config
            .nats
            .clone()
            .expect("Nats configuration must be provided to connect to the Nats cluster");

        let addresses = nats_config.client.addresses.clone();
        let subject = nats_config.subject.clone();
        let matcher_addr_clone = matcher_addr.clone();

        actix::spawn(async move {
            subscribe_to_nats(nats_config, message_queue_size, move |msg| {
                let event = serde_json::from_slice(&msg.msg)
                    .map_err(|err| TornadoCommonActorError::SerdeError { message: format! {"{}", err} })?;
                trace!("NatsSubscriberActor - event from message received: {:#?}", event);
                matcher_addr_clone.try_send(EventMessage { event }).unwrap_or_else(|err| error!("NatsSubscriberActor - Error while sending EventMessage to MatcherActor. Error: {}", err));
                Ok(())
            })
            .await
            .map(|_| {
                info!(
                    "NATS connection started at [{:#?}]. Listening for incoming events on subject [{}]",
                    addresses, subject
                );
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

        actix::spawn(async move {
            listen_to_tcp(tcp_address.clone(), message_queue_size, move |msg| {
                let json_matcher_addr_clone = json_matcher_addr_clone.clone();
                JsonEventReaderActor::start_new(msg, message_queue_size, move |event| {
                    json_matcher_addr_clone.try_send(EventMessage { event }).unwrap_or_else(|err| error!("JsonEventReaderActor - Error while sending EventMessage to MatcherActor. Error: {}", err));
                });
            })
            .await
            .map(|_| {
                info!("Started TCP server at [{}]. Listening for incoming events", tcp_address);
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
        let daemon_config = daemon_config.clone();

        let auth_api = ApiData { auth: auth_service.clone(), api: () };
        let config_api = ApiData {
            auth: auth_service.clone(),
            api: ConfigApi::new(api_handler.clone(), matcher_config.clone()),
        };
        let event_api = ApiData {
            auth: auth_service.clone(),
            api: EventApi::new(api_handler.clone(), matcher_config.clone()),
        };

        App::new()
            .wrap(Logger::default())
            .wrap(Cors::new().max_age(3600).finish())
            .service(
                web::scope("/api")
                    .app_data(
                        // Json extractor configuration for this resource.
                        web::JsonConfig::default()
                            .limit(daemon_config.web_max_json_payload_size.unwrap_or(MAX_JSON_PAYLOAD_SIZE)) // Limit request payload size in byte
                    )
                    .service(tornado_engine_api::auth::web::build_auth_endpoints(auth_api))
                    .service(tornado_engine_api::config::web::build_config_endpoints(config_api))
                    .service(tornado_engine_api::event::web::build_event_endpoints(event_api)),
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
