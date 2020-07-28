use crate::config::{EventConfig, NatsJsonCollectorConfig, TopicConfig, TornadoConnectionChannel};
use actix::Recipient;
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

pub mod config;

const DEFAULT_PAYLOAD_DATA_KEY: &str = "data";
const DEFAULT_PAYLOAD_DATA_EXPRESSION: &str = "${@}";

pub async fn start(
    nats_json_collector_config: NatsJsonCollectorConfig,
    topics_config: Vec<TopicConfig>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let nats_config = nats_json_collector_config.nats_client;

    let recipient = match nats_json_collector_config.tornado_connection_channel {
        TornadoConnectionChannel::Nats { nats_subject } => {
            info!("Connect to Tornado through NATS subject [{}]", nats_subject);

            let nats_publisher_config =
                NatsPublisherConfig { client: nats_config.clone(), subject: nats_subject };

            let actor_address = NatsPublisherActor::start_new(
                nats_publisher_config,
                nats_json_collector_config.message_queue_size,
            )?;
            actor_address.recipient()
        }
        TornadoConnectionChannel::TCP { tcp_socket_ip, tcp_socket_port } => {
            info!("Connect to Tornado through TCP socket");
            // Start TcpWriter
            let tornado_tcp_address = format!("{}:{}", tcp_socket_ip, tcp_socket_port,);

            let actor_address = TcpClientActor::start_new(
                tornado_tcp_address,
                nats_json_collector_config.message_queue_size,
            );
            actor_address.recipient()
        }
    };

    subscribe_to_topics(
        nats_config,
        recipient,
        nats_json_collector_config.message_queue_size,
        topics_config,
    )
    .await
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
                debug!("Topic [{}] called", topic);

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
            payload.insert(
                DEFAULT_PAYLOAD_DATA_KEY.to_owned(),
                Value::Text(DEFAULT_PAYLOAD_DATA_EXPRESSION.to_owned()),
            );
            payload
        }),
    }
}
#[cfg(test)]
mod test {

    use super::*;
    use maplit::*;

    #[test]
    fn should_return_default_jmespath_collector_config() {
        let jmespath_config_1 = build_jmespath_collector_config(None, "topic_name");
        let jmespath_config_2 = build_jmespath_collector_config(
            Some(EventConfig { payload: None, event_type: None }),
            "topic_name",
        );

        assert_eq!(jmespath_config_2, jmespath_config_1);
        assert_eq!("topic_name", &jmespath_config_1.event_type);

        assert_eq!(
            hashmap!(
                "data".to_owned() => Value::Text("${@}".to_owned()),
            ),
            jmespath_config_1.payload
        );
    }

    #[test]
    fn should_return_default_payload_in_jmespath_collector_config() {
        let jmespath_config = build_jmespath_collector_config(
            Some(EventConfig { event_type: Some("event_type_2".to_owned()), payload: None }),
            "topic_name_1",
        );

        assert_eq!("event_type_2", &jmespath_config.event_type);

        assert_eq!(
            hashmap!(
                "data".to_owned() => Value::Text("${@}".to_owned()),
            ),
            jmespath_config.payload
        );
    }

    #[test]
    fn should_return_default_event_type_in_jmespath_collector_config() {
        let payload = hashmap!(
            "key".to_owned() => Value::Text("value".to_owned()),
        );
        let jmespath_config = build_jmespath_collector_config(
            Some(EventConfig { event_type: None, payload: Some(payload.clone()) }),
            "topic_name_1",
        );

        assert_eq!("topic_name_1", &jmespath_config.event_type);

        assert_eq!(payload, jmespath_config.payload);
    }
}
