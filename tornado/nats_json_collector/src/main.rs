use crate::config::{TopicConfig, TornadoConnectionChannel};
use actix::dev::ToEnvelope;
use actix::{Actor, Addr, System, Recipient};
use chrono::prelude::Local;
use log::*;
use tornado_collector_common::CollectorError;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::nats_publisher::{NatsPublisherActor, NatsPublisherConfig, NatsClientConfig};
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common::TornadoError;
use tornado_common_api::{Event, Value};
use tornado_common_logger::setup_logger;
use tornado_common::actors::nats_subscriber::{NatsSubscriberConfig, subscribe_to_nats};

mod config;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");
    let topics_dir =
        arg_matches.value_of("topics-dir").expect("topics-dir should be provided");

    let collector_config = config::build_config(&config_dir)?;

    setup_logger(&collector_config.logger)?;

    let full_topics_dir = format!("{}/{}", &config_dir, &topics_dir);
    let topics_config = config::read_topics_from_config(&full_topics_dir)?;

    let nats_config = collector_config.nats_json_collector.nats_client;

    let recipient = match collector_config.nats_json_collector.tornado_connection_channel {
            TornadoConnectionChannel::Nats { nats_subject } => {
                info!("Connect to Tornado through NATS subject [{}]", nats_subject);

                let nats_publisher_config = NatsPublisherConfig {
                    client: nats_config.clone(),
                    subject: nats_subject
                };

                let actor_address = NatsPublisherActor::start_new(
                    nats_publisher_config,
                    collector_config.nats_json_collector.message_queue_size,
                )?;
                actor_address.recipient()
            }
            TornadoConnectionChannel::TCP { tcp_socket_ip, tcp_socket_port } => {
                info!("Connect to Tornado through TCP socket");
                // Start TcpWriter
                let tornado_tcp_address = format!("{}:{}", tcp_socket_ip, tcp_socket_port,);

                let actor_address = TcpClientActor::start_new(
                    tornado_tcp_address,
                    collector_config.nats_json_collector.message_queue_size,
                );
                actor_address.recipient()
            }
        };

    subscribe_to_topics(nats_config, recipient, collector_config.nats_json_collector.message_queue_size, topics_config).await?;

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();

    Ok(())
}

async fn subscribe_to_topics(nats_config: NatsClientConfig, recipient: Recipient<EventMessage>, message_queue_size: usize, topics_config: Vec<TopicConfig>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    for topic_config in topics_config {
        for topic in topic_config.nats_topics {
            info!("Subscribe to NATS topic [{}]", topic);
        topic_config.collector_config.clone();
            let nats_subscriber_config = NatsSubscriberConfig {
                subject: topic,
                client: nats_config.clone()
            };
            subscribe_to_nats(nats_subscriber_config, message_queue_size, |_: Value| {
                Ok(())
            }).await?;
        }
    }

    Ok(())
}

/*
fn create_app<R: Fn(Event) + 'static, F: Fn() -> R>(
    webhooks_config: Vec<TopicConfig>,
    factory: F,
) -> Result<Scope, CollectorError> {
    let mut scope = web::scope("");
    scope = scope.service(web::resource("/ping").route(web::get().to(pong)));

    for config in webhooks_config {
        let id = config.id.clone();
        let handler = handler::Handler {
            id: config.id.clone(),
            token: config.token,
            collector: JMESPathEventCollector::build(config.collector_config).map_err(|err| {
                CollectorError::CollectorCreationError {
                    message: format!(
                        "Cannot create collector for webhook with id [{}]. Err: {}",
                        id, err
                    ),
                }
            })?,
            callback: factory(),
        };

        let path = format!("/event/{}", config.id);
        debug!("Creating endpoint: [{}]", &path);

        let new_scope = web::scope(&path)
            .data(handler)
            .service(web::resource("").route(web::post().to(handle::<R>)));

        scope = scope.service(new_scope);
    }

    Ok(scope)
}

async fn start_http_server<A: Actor + actix::Handler<EventMessage>>(
    actor_address: Addr<A>,
    webhooks_config: Vec<TopicConfig>,
    bind_address: String,
    port: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    <A as Actor>::Context: ToEnvelope<A, tornado_common::actors::message::EventMessage>,
{
    HttpServer::new(move || {
        App::new().service(
            create_app(webhooks_config.clone(), || {
                let clone = actor_address.clone();
                move |event| clone.do_send(EventMessage { event })
            })
            // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
            .unwrap_or_else(|err| {
                error!("Cannot create the webhook handlers. Err: {}", err);
                std::process::exit(1);
            }),
        )
    })
    .bind(format!("{}:{}", bind_address, port))
    // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
    .unwrap_or_else(|err| {
        error!("Server cannot start on port {}. Err: {}", port, err);
        std::process::exit(1);
    })
    .run()
    .await?;

    Ok(())
}

async fn pong() -> impl Responder {
    let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
    let created_ms: String = dt.to_rfc3339();
    format!("pong - {}", created_ms)
}

async fn handle<F: Fn(Event) + 'static>(
    body: String,
    query: Query<TokenQuery>,
    handler: Data<Handler<F>>,
) -> Result<String, HandlerError> {
    let received_token = &query.token;
    handler.handle(&body, received_token)
}

 */

#[cfg(test)]
mod test {

    use super::*;
    use actix_web::{http, test};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;

    #[actix_rt::test]
    async fn ping_should_return_pong() {
        // Arrange
        let mut srv =
            test::init_service(App::new().service(create_app(vec![], || |_| {}).unwrap())).await;

        // Act
        let request = test::TestRequest::get().uri("/ping").to_request();

        let response = test::read_response(&mut srv, request).await;

        // Assert
        let body = std::str::from_utf8(&response).unwrap();

        assert!(body.contains("pong - "));
    }

    #[actix_rt::test]
    async fn should_create_a_path_per_webhook() {
        // Arrange
        let mut webhooks_config = vec![];
        webhooks_config.push(TopicConfig {
            id: "hook_1".to_owned(),
            token: "hook_1_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "hook_1_type".to_owned(),
                payload: HashMap::new(),
            },
        });
        webhooks_config.push(TopicConfig {
            id: "hook_2".to_owned(),
            token: "hook_2_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "hook_2_type".to_owned(),
                payload: HashMap::new(),
            },
        });
        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), || |_| {}).unwrap()),
        )
        .await;

        // Act
        let request_1 = test::TestRequest::post()
            .uri("/event/hook_1?token=hook_1_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response_1 = test::read_response(&mut srv, request_1).await;

        let request_2 = test::TestRequest::post()
            .uri("/event/hook_2?token=hook_2_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response_2 = test::read_response(&mut srv, request_2).await;

        // Assert
        let body_1 = std::str::from_utf8(&response_1).unwrap();
        assert_eq!("hook_1", body_1);

        let body_2 = std::str::from_utf8(&response_2).unwrap();
        assert_eq!("hook_2", body_2);
    }

    #[actix_rt::test]
    async fn should_accept_calls_only_if_token_matches() {
        // Arrange
        let mut webhooks_config = vec![];
        webhooks_config.push(TopicConfig {
            id: "hook_1".to_owned(),
            token: "hook_1_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "hook_1_type".to_owned(),
                payload: HashMap::new(),
            },
        });
        webhooks_config.push(TopicConfig {
            id: "hook_2".to_owned(),
            token: "hook_2_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "hook_2_type".to_owned(),
                payload: HashMap::new(),
            },
        });
        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), || |_| {}).unwrap()),
        )
        .await;

        // Act
        let request_1 = test::TestRequest::post()
            .uri("/event/hook_1?token=hook_1_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response_1 = test::call_service(&mut srv, request_1).await;

        let request_2 = test::TestRequest::post()
            .uri("/event/hook_2?token=WRONG_TOKEN")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response_2 = test::call_service(&mut srv, request_2).await;

        // Assert
        assert!(response_1.status().is_success());
        assert_eq!(http::StatusCode::UNAUTHORIZED, response_2.status());
    }

    #[actix_rt::test]
    async fn should_call_the_callback_on_each_event() {
        // Arrange
        let mut webhooks_config = vec![];

        webhooks_config.push(TopicConfig {
            id: "hook_1".to_owned(),
            token: "hook_1_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "${map.first}".to_owned(),
                payload: HashMap::new(),
            },
        });

        let event = Arc::new(Mutex::new(None));
        let event_clone = event.clone();

        let mut srv = test::init_service(
            App::new().service(
                create_app(webhooks_config.clone(), || {
                    let clone = event.clone();
                    move |evt| {
                        let mut wrapper = clone.lock().unwrap();
                        *wrapper = Some(evt)
                    }
                })
                .unwrap(),
            ),
        )
        .await;

        // Act
        let request_1 = test::TestRequest::post()
            .uri("/event/hook_1?token=hook_1_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload(
                r#"{
                    "map" : {
                        "first": "webhook_event"
                    }
                }"#,
            )
            .to_request();
        let response_1 = test::read_response(&mut srv, request_1).await;

        // Assert
        let body_1 = std::str::from_utf8(&response_1).unwrap();
        assert_eq!("hook_1", body_1);

        let value = event_clone.lock().unwrap();
        assert_eq!("webhook_event", value.as_ref().unwrap().event_type)
    }

    #[actix_rt::test]
    async fn should_return_404_if_hook_does_not_exists() {
        // Arrange
        let mut webhooks_config = vec![];

        webhooks_config.push(TopicConfig {
            id: "hook_1".to_owned(),
            token: "hook_1_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "${map.first}".to_owned(),
                payload: HashMap::new(),
            },
        });

        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), || |_| {}).unwrap()),
        )
        .await;

        // Act
        let request = test::TestRequest::post()
            .uri("/event/hook_2?token=hook_2_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(http::StatusCode::NOT_FOUND, response.status());
    }

    #[actix_rt::test]
    async fn should_return_405_if_get_instead_of_post() {
        // Arrange
        let mut webhooks_config = vec![];

        webhooks_config.push(TopicConfig {
            id: "hook_1".to_owned(),
            token: "hook_1_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "${map.first}".to_owned(),
                payload: HashMap::new(),
            },
        });

        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), || |_| {}).unwrap()),
        )
        .await;

        // Act
        let request = test::TestRequest::get()
            .uri("/event/hook_1?token=hook_1_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .to_request();
        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(http::StatusCode::METHOD_NOT_ALLOWED, response.status());
    }

    #[actix_rt::test]
    async fn should_url_encode_id_and_token() {
        // Arrange
        let mut webhooks_config = vec![];

        webhooks_config.push(TopicConfig {
            id: "hook with space".to_owned(),
            token: "token&#?=".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "type".to_owned(),
                payload: HashMap::new(),
            },
        });

        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), || |_| {}).unwrap()),
        )
        .await;

        // Act
        let request = test::TestRequest::post()
            .uri("/event/hook%20with%20space?token=token%26%23%3F%3D")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(http::StatusCode::OK, response.status());
    }
}
