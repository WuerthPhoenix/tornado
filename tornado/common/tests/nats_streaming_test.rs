#![cfg(feature = "nats_streaming")]

use std::sync::{Arc, Mutex};
use tokio::time;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::nats_streaming_publisher::{
    NatsPublisherActor, StanBaseConfig, StanPublisherConfig,
};
use tornado_common::actors::nats_streaming_subscriber::{
    subscribe_to_nats_streaming, StanSubscriberConfig,
};
use tornado_common_api::Event;

const BASE_ADDRESS: &str = "127.0.0.1:4222";

// This test requires a running NATS streaming server listening on BASE_ADDRESS
#[actix_rt::test]
async fn should_publish_to_nats_streaming() {
    start_logger();

    let random: u8 = rand::random();
    let event = Event::new(format!("event_type_{}", random));
    let subject = format!("test_subject_{}", random);

    let received = Arc::new(Mutex::new(None));

    let received_clone = received.clone();

    subscribe_to_nats_streaming(
        StanSubscriberConfig {
            base: StanBaseConfig {
                subject: subject.to_owned(),
                cluster_id: "test-cluster".to_owned(),
                client_id: "test-client-1".to_owned(),
                addresses: vec![BASE_ADDRESS.to_owned()],
            },
            queue_group: None,
            durable_name: None,
        },
        10000,
        move |event| {
            let mut lock = received_clone.lock().unwrap();
            *lock = Some(event);
            Ok(())
        },
    )
    .await
    .unwrap();

    let publisher = NatsPublisherActor::start_new(
        StanPublisherConfig {
            base: StanBaseConfig {
                subject: subject.to_owned(),
                cluster_id: "test-cluster".to_owned(),
                client_id: "test-client-2".to_owned(),
                addresses: vec![BASE_ADDRESS.to_owned()],
            },
        },
        10,
    )
    .await
    .unwrap();
    publisher.do_send(EventMessage { event: event.clone() });

    time::delay_until(time::Instant::now() + time::Duration::new(2, 0)).await;

    let received_event = received.lock().unwrap();
    assert!(received_event.is_some());
    assert_eq!(&event, received_event.as_ref().unwrap());
}

fn start_logger() {
    println!("Init logger");

    let conf = tornado_common_logger::LoggerConfig {
        level: String::from("trace"),
        stdout_output: true,
        file_output_path: None,
    };
    tornado_common_logger::setup_logger(&conf).unwrap();
}
