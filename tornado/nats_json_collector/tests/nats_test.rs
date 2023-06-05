#![cfg(feature = "nats_integration_tests")]
//
// WARN: This tests require docker on the host machine
//

use actix::Addr;
use rand::Rng;
use serde_json::json;
use testcontainers::images::generic::GenericImage;
use testcontainers::*;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::nats_publisher::{
    NatsClientConfig, NatsPublisherActor, NatsPublisherConfig,
};
use tornado_common::actors::nats_subscriber::{subscribe_to_nats, NatsSubscriberConfig};
use tornado_common_api::{Event, Map, TracedEvent, Value};
use tornado_nats_json_collector::config::{NatsJsonCollectorConfig, TornadoConnectionChannel};
use tornado_nats_json_collector::*;
use tracing::Span;

fn new_nats_docker_container(
    docker: &clients::Cli,
) -> (Container<'_, clients::Cli, GenericImage>, u16) {
    let image = images::generic::GenericImage::new("nats:2.1-alpine");
    let node = docker
        .run(image.with_wait_for(images::generic::WaitFor::message_on_stderr("Server is ready")));
    let nats_port = node.get_host_port(4222).unwrap();
    (node, nats_port)
}

#[actix_rt::test]
async fn should_subscribe_to_nats_topics() {
    // Arrange
    let docker = clients::Cli::default();
    let (_node, nats_port) = new_nats_docker_container(&docker);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    let random: u32 = rand::thread_rng().gen();
    let tornado_nats_subject = format!("tornado_subject_{}", random);

    let config = NatsJsonCollectorConfig {
        message_queue_size: 100,
        nats_client: NatsClientConfig { auth: None, addresses: vec![nats_address.to_owned()] },
        tornado_connection_channel: TornadoConnectionChannel::Nats {
            nats_subject: tornado_nats_subject.clone(),
        },
    };

    let topics_config = config::read_topics_from_config("./config/topics").unwrap();
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    // This subscriber gets the messages sent to tornado
    subscribe_to_nats(
        NatsSubscriberConfig {
            client: NatsClientConfig { addresses: vec![nats_address.to_owned()], auth: None },
            subject: tornado_nats_subject.clone(),
        },
        10000,
        move |msg| {
            let event: Event = serde_json::from_slice(&msg.msg.data).unwrap();
            sender.send(event).unwrap();
            Ok(())
        },
    )
    .await
    .unwrap();

    // Act
    start(config, topics_config).await.unwrap();

    // Assert
    {
        let vsphere_publisher = new_publisher(nats_address.to_owned(), "vsphere".to_owned()).await;

        let event_type = format!("event_type_{}", random);
        let mut source = Event::new(event_type.clone());
        let mut metadata = Map::new();
        metadata.insert("some_metadata11".to_owned(), Value::Number(serde_json::Number::from(1)));
        metadata.insert("trace_context".to_owned(), Value::Object(serde_json::Map::new()));
        source.metadata = metadata;

        vsphere_publisher
            .do_send(EventMessage(TracedEvent { event: source.clone(), span: Span::current() }));

        let received = receiver.recv().await.unwrap();
        assert_eq!("vmd", received.event_type);
        assert!(received.created_ms > 0);

        let source: Value = json!(source);
        let mut payload = Map::new();
        payload.insert("metrics".to_owned(), source);
        assert_eq!(payload, received.payload);
    }

    {
        let another_topic_publisher =
            new_publisher(nats_address.to_owned(), "another_topic".to_owned()).await;

        let event_type = format!("another_event_type_{}", random);
        let mut source = Event::new(event_type.clone());
        let mut metadata = Map::new();
        metadata.insert("some_metadata".to_owned(), Value::Number(serde_json::Number::from(1)));
        metadata.insert("trace_context".to_owned(), Value::Object(serde_json::Map::new()));
        source.metadata = metadata;
        another_topic_publisher
            .do_send(EventMessage(TracedEvent { event: source.clone(), span: Span::current() }));

        let received = receiver.recv().await.unwrap();
        assert_eq!("vmd", received.event_type);
        assert!(received.created_ms > 0);

        let source: Value = json!(source);
        let mut payload = Map::new();
        payload.insert("metrics".to_owned(), source);
        assert_eq!(payload, received.payload);
    }

    {
        let vsphere_simple_publisher =
            new_publisher(nats_address.to_owned(), "vsphere_simple".to_owned()).await;

        let event_type = format!("another_event_type_{}", random);
        let mut source = Event::new(event_type.clone());
        let mut metadata = Map::new();
        metadata.insert("some_metadata1".to_owned(), Value::Number(serde_json::Number::from(1)));
        metadata.insert("trace_context".to_owned(), Value::Object(serde_json::Map::new()));
        source.metadata = metadata;
        vsphere_simple_publisher
            .do_send(EventMessage(TracedEvent { event: source.clone(), span: Span::current() }));

        let received = receiver.recv().await.unwrap();
        assert_eq!("vsphere_simple", &received.event_type);
        assert!(received.created_ms > 0);

        let source: Value = json!(source);
        let mut payload = Map::new();
        payload.insert("data".to_owned(), source);
        assert_eq!(payload, received.payload);
    }
}

async fn new_publisher(nats_address: String, subject: String) -> Addr<NatsPublisherActor> {
    NatsPublisherActor::start_new(
        NatsPublisherConfig {
            client: NatsClientConfig { addresses: vec![nats_address], auth: None },
            subject,
        },
        10,
    )
    .await
    .unwrap()
}
