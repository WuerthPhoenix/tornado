use crate::config::{EventConfig, TopicConfig, TornadoConnectionChannel};
use actix::{Recipient, System};
use log::*;
use std::collections::HashMap;
use tornado_collector_common::{Collector, CollectorError};
use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common::actors::message::{EventMessage, TornadoCommonActorError};
use tornado_common::actors::nats_publisher::{
    NatsClientConfig, NatsPublisherActor, NatsPublisherConfig,
};
use tornado_common::actors::nats_subscriber::{subscribe_to_nats, NatsSubscriberConfig};
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common_api::Value;
use tornado_common_logger::setup_logger;

mod config;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");
    let topics_dir = arg_matches.value_of("topics-dir").expect("topics-dir should be provided");

    let collector_config = config::build_config(&config_dir)?;

    setup_logger(&collector_config.logger)?;

    let full_topics_dir = format!("{}/{}", &config_dir, &topics_dir);
    let topics_config = config::read_topics_from_config(&full_topics_dir)?;

    let nats_config = collector_config.nats_json_collector.nats_client;

    let recipient = match collector_config.nats_json_collector.tornado_connection_channel {
        TornadoConnectionChannel::Nats { nats_subject } => {
            info!("Connect to Tornado through NATS subject [{}]", nats_subject);

            let nats_publisher_config =
                NatsPublisherConfig { client: nats_config.clone(), subject: nats_subject };

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

    subscribe_to_topics(
        nats_config,
        recipient,
        collector_config.nats_json_collector.message_queue_size,
        topics_config,
    )
    .await?;

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();

    Ok(())
}

async fn subscribe_to_topics(
    nats_config: NatsClientConfig,
    recipient: Recipient<EventMessage>,
    message_queue_size: usize,
    topics_config: Vec<TopicConfig>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    for topic_config in topics_config {
        for topic in topic_config.nats_topics {
            info!("Subscribe to NATS topic [{}]", topic);

            let jmespath_collector_config =
                build_jmespath_collector_config(topic_config.collector_config.clone(), &topic);
            let jmespath_collector = JMESPathEventCollector::build(jmespath_collector_config)
                .map_err(|err| CollectorError::CollectorCreationError {
                    message: format!("Cannot create collector for topic [{}]. Err: {}", topic, err),
                })?;

            let nats_subscriber_config =
                NatsSubscriberConfig { subject: topic.clone(), client: nats_config.clone() };

            let recipient_clone = recipient.clone();
            subscribe_to_nats(nats_subscriber_config, message_queue_size, move |data| {
                trace!("Topic [{}] called", topic);

                let event = std::str::from_utf8(&data.msg)
                    .map_err(|err| CollectorError::EventCreationError {
                        message: format!("{}", err),
                    })
                    .and_then(|text| jmespath_collector.to_event(text))
                    .map_err(|err| TornadoCommonActorError::GenericError {
                        message: format!("{}", err),
                    })?;

                recipient_clone.try_send(EventMessage { event }).map_err(|err| {
                    TornadoCommonActorError::GenericError { message: format!("{}", err) }
                })
            })
            .await?;
        }
    }

    Ok(())
}

fn build_jmespath_collector_config(
    collector_config: Option<EventConfig>,
    topic: &str,
) -> JMESPathEventCollectorConfig {
    let collector_config =
        collector_config.unwrap_or_else(|| EventConfig { event_type: None, payload: None });

    JMESPathEventCollectorConfig {
        event_type: collector_config.event_type.unwrap_or_else(|| topic.to_owned()),
        payload: collector_config.payload.unwrap_or_else(|| {
            let mut payload = HashMap::new();
            payload.insert("data".to_owned(), Value::Text("${@}".to_owned()));
            payload
        }),
    }
}
/*
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
*/