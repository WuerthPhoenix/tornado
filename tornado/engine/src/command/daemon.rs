use crate::actor::dispatcher::{ActixEventBus, DispatcherActor};
use crate::actor::foreach::{ForEachExecutorActor, ForEachExecutorActorInitMessage};
use crate::actor::matcher::{EventMessage, MatcherActor};
use crate::api::runtime_config::RuntimeConfigApiHandlerImpl;
use crate::api::MatcherApiHandler;
use crate::config;
use crate::config::build_config;
use crate::monitoring::endpoint::monitoring_endpoints;
use crate::monitoring::metrics::{
    TornadoMeter, EVENT_SOURCE_LABEL_KEY, EVENT_TYPE_LABEL_KEY, TORNADO_APP,
};
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use log::*;
use serde_json::json;
use std::rc::Rc;
use std::sync::Arc;
use tornado_common::actors::command::CommandExecutorActor;
use tornado_common::actors::json_event_reader::JsonEventReaderActor;
use tornado_common::actors::message::{ActionMessage, TornadoCommonActorError, TornadoNatsMessage};
use tornado_common::actors::nats_subscriber::subscribe_to_nats;
use tornado_common::actors::tcp_server::listen_to_tcp;
use tornado_common::command::pool::{CommandMutPool, CommandPool};
use tornado_common::command::retry::RetryCommand;
use tornado_common::command::{StatefulExecutorCommand, StatelessExecutorCommand};
use tornado_common::metrics::{ActionMeter, ACTION_ID_LABEL_KEY};
use tornado_common::TornadoError;
use tornado_common_logger::elastic_apm::DEFAULT_APM_SERVER_CREDENTIALS_FILENAME;
use tornado_common_logger::setup_logger;
use tornado_common_metrics::Metrics;
use tornado_engine_api::auth::auth_v2::AuthServiceV2;
use tornado_engine_api::auth::{roles_map_to_permissions_map, AuthService};
use tornado_engine_api::config::api::ConfigApi;
use tornado_engine_api::event::api::EventApi;
use tornado_engine_api::event::api_v2::EventApiV2;
use tornado_engine_api::model::{ApiData, ApiDataV2};
use tornado_engine_api::runtime_config::api::RuntimeConfigApi;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tracing_actix_web::TracingLogger;
use tornado_common_metrics::opentelemetry::global;
use tracing_opentelemetry::OpenTelemetrySpanExt;

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
    let mut global_config = build_config(config_dir)?;
    let apm_server_api_credentials_filepath =
        format!("{}/{}", config_dir, DEFAULT_APM_SERVER_CREDENTIALS_FILENAME);
    // Get the result and log the error later because the logger is not available yet
    let apm_credentials_read_result = global_config
        .logger
        .tracing_elastic_apm
        .read_apm_server_api_credentials_if_not_set(&apm_server_api_credentials_filepath);

    let logger_guard = Arc::new(setup_logger(global_config.logger)?);
    if let Err(apm_credentials_read_error) = apm_credentials_read_result {
        warn!("{:?}", apm_credentials_read_error);
    }

    let configs = config::parse_config_files(config_dir, rules_dir, drafts_dir)?;

    // start system
    let metrics = Arc::new(Metrics::new(TORNADO_APP));
    let tornado_meter = Arc::new(TornadoMeter::default());
    let action_meter = Arc::new(ActionMeter::new(TORNADO_APP));

    let daemon_config = global_config.tornado.daemon;
    let thread_pool_config = daemon_config.thread_pool_config.clone().unwrap_or_default();
    let threads_per_queue = thread_pool_config.get_threads_count();
    info!(
        "Starting Tornado daemon with {} threads per queue. Thread config: {:?}",
        threads_per_queue, thread_pool_config
    );

    let retry_strategy = daemon_config.retry_strategy.clone();
    info!("Tornado global retry strategy: {:?}", retry_strategy);

    let message_queue_size = daemon_config.message_queue_size;

    // Start ForEach executor actor
    let foreach_executor_addr = ForEachExecutorActor::start_new(message_queue_size);

    let archive_action_meter = action_meter.clone();
    // Start archive executor actor
    let archive_executor_addr = {
        let archive_config = configs.archive_executor_config.clone();
        CommandExecutorActor::start_new(
            message_queue_size,
            Rc::new(RetryCommand::new(
                retry_strategy.clone(),
                CommandMutPool::new(1, move || {
                    StatefulExecutorCommand::new(
                        archive_action_meter.clone(),
                        tornado_executor_archive::ArchiveExecutor::new(&archive_config),
                    )
                }),
            )),
            action_meter.clone(),
        )
    };

    // Start script executor actor
    let script_executor_addr = {
        let executor = tornado_executor_script::ScriptExecutor::new();
        let stateless_executor_command =
            StatelessExecutorCommand::new(action_meter.clone(), executor);
        CommandExecutorActor::start_new(
            message_queue_size,
            Rc::new(RetryCommand::new(
                retry_strategy.clone(),
                CommandPool::new(threads_per_queue, stateless_executor_command),
            )),
            action_meter.clone(),
        )
    };

    // Start logger executor actor
    let logger_executor_addr = {
        let executor = tornado_executor_logger::LoggerExecutor::new();
        let stateless_executor_command =
            StatelessExecutorCommand::new(action_meter.clone(), executor);
        CommandExecutorActor::start_new(
            message_queue_size,
            Rc::new(RetryCommand::new(
                retry_strategy.clone(),
                CommandPool::new(threads_per_queue, stateless_executor_command),
            )),
            action_meter.clone(),
        )
    };

    // Start elasticsearch executor actor
    let elasticsearch_executor_addr = {
        let es_authentication = configs.elasticsearch_executor_config.default_auth.clone();
        let executor =
            tornado_executor_elasticsearch::ElasticsearchExecutor::new(es_authentication)
                .await
                .expect("Cannot start the Elasticsearch Executor");
        let stateless_executor_command =
            StatelessExecutorCommand::new(action_meter.clone(), executor);
        CommandExecutorActor::start_new(
            message_queue_size,
            Rc::new(RetryCommand::new(
                retry_strategy.clone(),
                CommandPool::new(threads_per_queue, stateless_executor_command),
            )),
            action_meter.clone(),
        )
    };

    // Start icinga2 executor actor
    let icinga2_executor_addr = {
        let executor =
            tornado_executor_icinga2::Icinga2Executor::new(configs.icinga2_executor_config.clone())
                .expect("Cannot start the Icinga2Executor Executor");
        let stateless_executor_command =
            StatelessExecutorCommand::new(action_meter.clone(), executor);
        CommandExecutorActor::start_new(
            message_queue_size,
            Rc::new(RetryCommand::new(
                retry_strategy.clone(),
                CommandPool::new(threads_per_queue, stateless_executor_command),
            )),
            action_meter.clone(),
        )
    };

    // Start director executor actor
    let director_client_config = configs.director_executor_config.clone();
    let director_executor_addr = {
        let executor =
            tornado_executor_director::DirectorExecutor::new(director_client_config.clone())
                .expect("Cannot start the DirectorExecutor Executor");
        let stateless_executor_command =
            StatelessExecutorCommand::new(action_meter.clone(), executor);
        CommandExecutorActor::start_new(
            message_queue_size,
            Rc::new(RetryCommand::new(
                retry_strategy.clone(),
                CommandPool::new(threads_per_queue, stateless_executor_command),
            )),
            action_meter.clone(),
        )
    };

    // Start monitoring executor actor
    let monitoring_executor_addr = {
        let executor = tornado_executor_monitoring::MonitoringExecutor::new(
            configs.icinga2_executor_config.clone(),
            configs.director_executor_config.clone(),
        )
        .expect("Cannot start the MonitoringExecutor Executor");
        let stateless_executor_command =
            StatelessExecutorCommand::new(action_meter.clone(), executor);
        CommandExecutorActor::start_new(
            message_queue_size,
            Rc::new(RetryCommand::new(
                retry_strategy.clone(),
                CommandPool::new(threads_per_queue, stateless_executor_command),
            )),
            action_meter.clone(),
        )
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
        let stateless_executor_command =
            StatelessExecutorCommand::new(action_meter.clone(), executor);
        CommandExecutorActor::start_new(
            message_queue_size,
            Rc::new(RetryCommand::new(
                retry_strategy.clone(),
                CommandPool::new(threads_per_queue, stateless_executor_command),
            )),
            action_meter.clone(),
        )
    };

    // Configure action dispatcher
    let foreach_executor_addr_clone = foreach_executor_addr.clone();
    let event_bus = {
        let event_bus = ActixEventBus {
            callback: move |action| {
                let action = Arc::new(action);
                let span = tracing::Span::current();
                let message = ActionMessage { action, span };

                action_meter
                    .actions_received_counter
                    .add(1, &[ACTION_ID_LABEL_KEY.string(message.action.id.to_owned())]);

                let send_result = match message.action.id.as_ref() {
                    "archive" => {
                        archive_executor_addr.try_send(message).map_err(|err| {
                            format!("Error sending message to 'archive' executor. Err: {:?}", err)
                        })
                    }
                    "icinga2" => {
                        icinga2_executor_addr.try_send(message).map_err(|err| {
                            format!("Error sending message to 'icinga2' executor. Err: {:?}", err)
                        })
                    }
                    "director" => {
                        director_executor_addr.try_send(message).map_err(|err| {
                            format!("Error sending message to 'director' executor. Err: {:?}", err)
                        })
                    }
                    ACTION_ID_MONITORING => {
                        monitoring_executor_addr.try_send(message).map_err(|err| {
                            format!(
                                "Error sending message to 'monitoring' executor. Err: {:?}",
                                err
                            )
                        })
                    }
                    ACTION_ID_SMART_MONITORING_CHECK_RESULT => {
                        smart_monitoring_check_result_executor_addr.try_send(message).map_err(|err| {
                            format!(
                                "Error sending message to 'smart_monitoring_check_result' executor. Err: {:?}",
                                err
                            )
                        })
                    }
                    "script" => {
                        script_executor_addr.try_send(message).map_err(|err| {
                            format!("Error sending message to 'script' executor. Err: {:?}", err)
                        })
                    }
                    ACTION_ID_FOREACH => foreach_executor_addr_clone
                        .try_send(message)
                        .map_err(|err| {
                            format!("Error sending message to 'foreach' executor. Err: {:?}", err)
                        }),
                    ACTION_ID_LOGGER => {
                        logger_executor_addr.try_send(message).map_err(|err| {
                            format!("Error sending message to 'logger' executor. Err: {:?}", err)
                        })
                    }
                    "elasticsearch" => elasticsearch_executor_addr
                        .try_send(message)
                        .map_err(|err| {
                            format!(
                                "Error sending message to 'elasticsearch' executor. Err: {:?}",
                                err
                            )
                        }),

                    _ => Err(format!("There are not executors for action id [{}]", &message.action.id)),
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
        dispatcher_addr.clone().recipient(),
        configs.matcher_config.clone(),
        message_queue_size,
        tornado_meter.clone(),
    )
    .await?;

    if daemon_config.is_nats_enabled() {
        info!("NATS connection is enabled. Starting it...");

        let nats_config = daemon_config
            .nats
            .clone()
            .expect("Nats configuration must be provided to connect to the Nats cluster");

        let addresses = nats_config.client.addresses.clone();
        let subject = nats_config.subject.clone();
        let matcher_addr_clone = matcher_addr.clone();
        let nats_extractors = daemon_config.nats_extractors.clone();

        let tornado_meter_nats = tornado_meter.clone();
        actix::spawn(async move {
            subscribe_to_nats(nats_config, message_queue_size, move |msg| {
                let meter_event_souce_label = EVENT_SOURCE_LABEL_KEY.string("nats");

                let tornado_nats_message: TornadoNatsMessage = serde_json::from_slice(&msg.msg.data)
                    .map_err(|err| {
                        tornado_meter_nats.invalid_events_received_counter.add(1, &[
                            meter_event_souce_label.clone(),
                        ]);
                        TornadoCommonActorError::SerdeError { message: format! {"{}", err} }
                    })?;
                debug!("NatsSubscriberActor - event from message received: {:#?}", tornado_nats_message);
                let event = tornado_nats_message.event;
                let parent_context_carrier = tornado_nats_message.trace_context;
                let parent_context = parent_context_carrier.map(|context| global::get_text_map_propagator(|prop| prop.extract(&context)));

                let span = tracing::error_span!("NatsSubscriberActor", command="daemon", trace_id=event.trace_id);
                if let Some(parent_context) = parent_context {
                    debug!("NatsSubscriberActor - span parent set to: {:?}", parent_context);
                    span.set_parent(parent_context)
                } else {
                    debug!("NatsSubscriberActor - no parent span received");
                }
                let _g = span.entered();

                tornado_meter_nats.events_received_counter.add(1, &[
                    meter_event_souce_label,
                    EVENT_TYPE_LABEL_KEY.string(event.event_type.to_owned()),
                ]);

                let mut event = json!(event);
                for extractor in &nats_extractors {
                    event = extractor.process(&msg.msg.subject, event)?;
                }

                matcher_addr_clone.try_send(EventMessage { event }).unwrap_or_else(|err| error!("NatsSubscriberActor - Error while sending EventMessage to MatcherActor. Error: {:?}", err));
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
                        "NATS connection failed started at [{:#?}], subject [{}]. Err: {:?}",
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
                .expect("'event_socket_port' must be provided to start the tornado TCP server")
        );
        let json_matcher_addr_clone = matcher_addr.clone();

        let tornado_meter_tcp = tornado_meter.clone();
        actix::spawn(async move {
            listen_to_tcp(tcp_address.clone(), message_queue_size, move |msg| {
                let tornado_meter = tornado_meter_tcp.clone();
                let json_matcher_addr_clone = json_matcher_addr_clone.clone();
                JsonEventReaderActor::start_new(msg, message_queue_size, move |event| {
                    tornado_meter.events_received_counter.add(1, &[
                        EVENT_SOURCE_LABEL_KEY.string("tcp"),
                        EVENT_TYPE_LABEL_KEY.string(event.event_type.to_owned()),
                    ]);
                    json_matcher_addr_clone.try_send(EventMessage { event: json!(event) }).unwrap_or_else(|err| error!("JsonEventReaderActor - Error while sending EventMessage to MatcherActor. Error: {:?}", err));
                });
            })
                .await
                .map(|_| {
                    info!("Started TCP server at [{}]. Listening for incoming events", tcp_address);
                })
                // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
                .unwrap_or_else(|err| {
                    error!("Cannot start TCP server at [{}]. Err: {:?}", tcp_address, err);
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
    let auth_service_v2 = AuthServiceV2::new(Arc::new(roles_map_to_permissions_map(
        daemon_config.auth.role_permissions.clone(),
    )));
    let api_handler = MatcherApiHandler::new(matcher_addr, tornado_meter.clone());
    let daemon_config = daemon_config.clone();
    let matcher_config = configs.matcher_config.clone();

    // Start API and monitoring endpoint
    let service_logger_guard = logger_guard.clone();
    let server_binding_result = HttpServer::new(move || {
        let daemon_config = daemon_config.clone();

        let v1_config_api = ApiData {
            auth: auth_service.clone(),
            api: ConfigApi::new(api_handler.clone(), matcher_config.clone()),
        };
        let v2_config_api = ApiDataV2 {
            auth: auth_service_v2.clone(),
            api: ConfigApi::new(api_handler.clone(), matcher_config.clone()),
        };
        let event_api = ApiData {
            auth: auth_service.clone(),
            api: EventApi::new(api_handler.clone(), matcher_config.clone()),
        };
        let event_api_v2 = ApiDataV2 {
            auth: auth_service_v2.clone(),
            api: EventApiV2::new(api_handler.clone(), matcher_config.clone()),
        };
        let runtime_config_api = ApiData {
            auth: auth_service.clone(),
            api: RuntimeConfigApi::new(RuntimeConfigApiHandlerImpl::new(
                service_logger_guard.clone(),
            )),
        };
        let metrics = metrics.clone();
        App::new()
            .wrap(Logger::default())
            .wrap(TracingLogger::default())
            .service(
                web::scope("/api")
                    .app_data(
                        // Json extractor configuration for this resource.
                        web::JsonConfig::default().limit(
                            daemon_config
                                .web_max_json_payload_size
                                .unwrap_or(MAX_JSON_PAYLOAD_SIZE),
                        ), // Limit request payload size in byte
                    )
                    .service(tornado_engine_api::config::web::build_config_endpoints(v1_config_api))
                    .service(tornado_engine_api::event::web::build_event_endpoints(event_api))
                    .service(
                        tornado_engine_api::runtime_config::web::build_runtime_config_endpoints(
                            runtime_config_api,
                        ),
                    )
                    .service(
                        web::scope("/v2_beta")
                            .service(tornado_engine_api::config::web::build_config_v2_endpoints(
                                v2_config_api,
                            ))
                            .service(tornado_engine_api::event::web::build_event_v2_endpoints(
                                event_api_v2,
                            )),
                    ),
            )
            .service(monitoring_endpoints(web::scope("/monitoring"), daemon_config, metrics))
    })
    .bind(format!("{}:{}", web_server_ip, web_server_port));

    match server_binding_result {
        Ok(server) => {
            server.run().await?;
            Ok(())
        }
        Err(err) => {
            let error = format!(
                "Web Server cannot start on address {}:{}. Err: {:?}",
                web_server_ip, web_server_port, err
            );
            error!("{}", error);
            Err(TornadoError::ExecutionError { message: error }.into())
        }
    }
}
