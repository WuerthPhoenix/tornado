#![cfg(feature = "nats")]

//
// This tests require docker on the host machine
//

use std::sync::{Arc, Mutex};
use testcontainers::images::generic::GenericImage;
use testcontainers::*;
use tokio::time;
use tokio::io;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::nats_publisher::{
    NatsClientConfig, NatsPublisherActor, NatsPublisherConfig,
};
use tornado_common::actors::nats_subscriber::{subscribe_to_nats, NatsSubscriberConfig};
use tornado_common_api::Event;
use log::*;

fn new_nats_docker_container(
    docker: &clients::Cli,
    port: Option<u16>
) -> (Container<'_, clients::Cli, GenericImage>, String) {

    let port = port.unwrap_or_else(|| port_check::free_local_port().unwrap());

    let node = docker.run_with_options(
        vec!["-d", "-p", &format!("{}:4222", port)],
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
    let (_node, nats_address) = new_nats_docker_container(&docker,None);

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

#[actix_rt::test]
async fn publisher_should_reprocess_the_event_if_nats_not_available_at_startup() {
    start_logger();
    let free_local_port = port_check::free_local_port().unwrap();

    let random: u8 = rand::random();
    let event = Event::new(format!("event_type_{}", random));
    let subject = format!("test_subject_{}", random);

    // Start a publisher and publish a message when Nats is not available
    let publisher = NatsPublisherActor::start_new(
        NatsPublisherConfig {
            client: NatsClientConfig { addresses: vec![format!("127.0.0.1:{}", free_local_port)] },
            subject: subject.to_owned(),
        },
        10,
    )
        .unwrap();
    publisher.do_send(EventMessage { event: event.clone() });

    info!("Publisher started");

    // Start NATS
    let docker = clients::Cli::default();
    let (_node, nats_address) = new_nats_docker_container(&docker, Some(free_local_port));

    let received = Arc::new(Mutex::new(None));
    let received_clone = received.clone();

    // Start a subscriber that should receive the message sent when NATS was down
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
