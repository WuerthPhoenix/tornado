#![cfg(feature = "nats")]

//
// This tests require docker on the host machine
//

use std::sync::{Arc, Mutex};
use testcontainers::images::generic::GenericImage;
use testcontainers::*;
use tokio::time;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::nats_publisher::{
    NatsClientConfig, NatsPublisherActor, NatsPublisherConfig,
};
use tornado_common::actors::nats_subscriber::{subscribe_to_nats, NatsSubscriberConfig};
use tornado_common_api::Event;

fn new_nats_docker_container(
    docker: &clients::Cli,
) -> (Container<'_, clients::Cli, GenericImage>, String) {
    let node = docker.run(
        images::generic::GenericImage::new("nats:2.1-alpine")
            .with_wait_for(images::generic::WaitFor::message_on_stderr("Server is ready")),
    );
    let nats_address = format!("127.0.0.1:{}", node.get_host_port(4222).unwrap());
    (node, nats_address)
}

#[actix_rt::test]
async fn should_publish_to_nats() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_address) = new_nats_docker_container(&docker);

    let random: u8 = rand::random();
    let event = Event::new(format!("event_type_{}", random));
    let subject = format!("test_subject_{}", random);

    let received = Arc::new(Mutex::new(None));

    let received_clone = received.clone();

    subscribe_to_nats(
        NatsSubscriberConfig {
            client: NatsClientConfig { addresses: vec![nats_address.to_owned()] },
            subject: subject.to_owned(),
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
        NatsPublisherConfig {
            client: NatsClientConfig { addresses: vec![nats_address.to_owned()] },
            subject: subject.to_owned(),
        },
        10,
    )
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
    if let Err(err) = tornado_common_logger::setup_logger(&conf) {
        println!("Warn: err starting logger: {}", err)
    };
}
