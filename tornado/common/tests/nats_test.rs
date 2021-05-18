#![cfg(feature = "nats_integration_tests")]

//
// This tests require docker on the host machine
//

use std::time::Duration;

use actix::clock::sleep;
use log::*;
use serial_test::serial;
use testcontainers::images::generic::GenericImage;
use testcontainers::*;
use tokio::time;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::nats_publisher::{
    NatsClientAuth, NatsClientConfig, NatsPublisherActor, NatsPublisherConfig,
};
use tornado_common::actors::nats_subscriber::{subscribe_to_nats, NatsSubscriberConfig};
use tornado_common_api::Event;

fn new_nats_docker_container(
    docker: &clients::Cli,
    port: Option<u16>,
    tls: bool,
) -> (Container<'_, clients::Cli, GenericImage>, u16) {
    let port = port.unwrap_or_else(|| port_check::free_local_port().unwrap());

    let mut image =
        images::generic::GenericImage::new("nats:2.1-alpine").with_mapped_port((port, 4222));
    if tls {
        image = image
            .with_volume(
                std::fs::canonicalize(&std::path::PathBuf::from("./test_resources"))
                    .unwrap()
                    .to_str()
                    .unwrap(),
                "/test_resources",
            )
            .with_args(vec![
                "nats-server".to_owned(),
                "--tls".to_owned(),
                "--tlscert".to_owned(),
                "/test_resources/nats-server.crt.pem".to_owned(),
                "--tlskey".to_owned(),
                "/test_resources/nats-server.key".to_owned(),
            ]);
    }

    let node = docker
        .run(image.with_wait_for(images::generic::WaitFor::message_on_stderr("Server is ready")));

    let nats_port = node.get_host_port(4222).unwrap();
    (node, nats_port)
}

#[actix_rt::test]
#[serial]
async fn official_nats_client_spike_test() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_port) = new_nats_docker_container(&docker, None, false);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    let random: u8 = rand::random();
    let subject = format!("test_subject_{}", random);

    let message = format!("message_{}", random);

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let nc_1 = async_nats::Options::new().connect(&nats_address).await.unwrap();
    let nc_2 = async_nats::connect(&nats_address).await.unwrap();

    let subscription = nc_2.subscribe(&subject).await.unwrap();
    actix::spawn(async move {
        let message = subscription.next().await;
        info!("message received: {:?}", message);
        sender.send(message.unwrap().data).unwrap();
    });

    nc_1.publish(&subject, &message).await.unwrap();

    assert_eq!(message.as_bytes(), &receiver.recv().await.unwrap());
}

#[actix_rt::test]
#[serial]
async fn nats_subscriber_should_receive_from_nats() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_port) = new_nats_docker_container(&docker, None, false);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    let random: u8 = rand::random();
    let subject = format!("test_subject_{}", random);
    let event = Event::new(format!("event_type_{}", random));

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let nc_1 = async_nats::connect(&nats_address).await.unwrap();

    subscribe_to_nats(
        NatsSubscriberConfig {
            client: NatsClientConfig { addresses: vec![nats_address.to_owned()], auth: None },
            subject: subject.to_owned(),
        },
        10000,
        move |event| {
            sender.send(event).unwrap();
            Ok(())
        },
    )
    .await
    .unwrap();

    sleep(Duration::from_millis(100)).await;

    info!("Sending message");
    nc_1.publish(&subject, &serde_json::to_vec(&event).unwrap()).await.unwrap();
    info!("Message sent");

    assert_eq!(serde_json::to_vec(&event).unwrap(), receiver.recv().await.unwrap().msg);
}

#[actix_rt::test]
#[serial]
async fn nats_publisher_should_publish_to_nats() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_port) = new_nats_docker_container(&docker, None, false);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    let random: u8 = rand::random();
    let subject = format!("test_subject_{}", random);
    let event = Event::new(format!("event_type_{}", random));

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let nc_1 = async_nats::connect(&nats_address).await.unwrap();

    let subscription = nc_1.subscribe(&subject).await.unwrap();
    actix::spawn(async move {
        let message = subscription.next().await;
        info!("message received: {:?}", message);
        sender.send(message.unwrap().data).unwrap();
    });

    sleep(Duration::from_millis(100)).await;

    let publisher = NatsPublisherActor::start_new(
        NatsPublisherConfig {
            client: NatsClientConfig { addresses: vec![nats_address.to_owned()], auth: None },
            subject: subject.to_owned(),
        },
        10,
    )
    .await
    .unwrap();
    publisher.do_send(EventMessage { event: event.clone() });

    assert_eq!(serde_json::to_vec(&event).unwrap(), receiver.recv().await.unwrap());
}

#[actix_rt::test]
#[serial]
async fn should_publish_to_nats() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_port) = new_nats_docker_container(&docker, None, false);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    let random: u8 = rand::random();
    let event = Event::new(format!("event_type_{}", random));
    let subject = format!("test_subject_{}", random);

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    subscribe_to_nats(
        NatsSubscriberConfig {
            client: NatsClientConfig { addresses: vec![nats_address.to_owned()], auth: None },
            subject: subject.to_owned(),
        },
        10000,
        move |event| {
            sender.send(event).unwrap();
            Ok(())
        },
    )
    .await
    .unwrap();

    let publisher = NatsPublisherActor::start_new(
        NatsPublisherConfig {
            client: NatsClientConfig { addresses: vec![nats_address.to_owned()], auth: None },
            subject: subject.to_owned(),
        },
        10,
    )
    .await
    .unwrap();
    publisher.do_send(EventMessage { event: event.clone() });

    assert_eq!(event, serde_json::from_slice(&receiver.recv().await.unwrap().msg).unwrap());
}

#[actix_rt::test]
#[serial]
async fn should_publish_to_nats_with_tls() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_port) = new_nats_docker_container(&docker, None, true);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    let random: u8 = rand::random();
    let event = Event::new(format!("event_type_{}", random));
    let subject = format!("test_subject_{}", random);

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let auth = Some(NatsClientAuth::Tls {
        path_to_pkcs12_bundle: "./test_resources/nats-client.pfx".to_string(),
        path_to_root_certificate: Some("./test_resources/root-ca.crt".to_string()),
        pkcs12_bundle_password: "".to_string(),
    });
    subscribe_to_nats(
        NatsSubscriberConfig {
            client: NatsClientConfig {
                addresses: vec![nats_address.to_owned()],
                auth: auth.clone(),
            },
            subject: subject.to_owned(),
        },
        10000,
        move |event| {
            sender.send(event).unwrap();
            Ok(())
        },
    )
    .await
    .unwrap();

    let publisher = NatsPublisherActor::start_new(
        NatsPublisherConfig {
            client: NatsClientConfig { addresses: vec![nats_address.to_owned()], auth },
            subject: subject.to_owned(),
        },
        10,
    )
    .await
    .unwrap();
    publisher.do_send(EventMessage { event: event.clone() });

    assert_eq!(event, serde_json::from_slice(&receiver.recv().await.unwrap().msg).unwrap());
}

#[actix_rt::test]
#[serial]
async fn publisher_should_reprocess_the_event_if_nats_is_not_available_at_startup() {
    start_logger();
    let free_local_port = port_check::free_local_port().unwrap();

    let random: u8 = rand::random();
    let event = Event::new(format!("event_type_{}", random));
    let subject = format!("test_subject_{}", random);

    // Start a publisher and publish a message when Nats is not available
    let publisher = NatsPublisherActor::start_new(
        NatsPublisherConfig {
            client: NatsClientConfig {
                addresses: vec![format!("127.0.0.1:{}", free_local_port)],
                auth: None,
            },
            subject: subject.to_owned(),
        },
        10,
    )
    .await
    .unwrap();
    publisher.do_send(EventMessage { event: event.clone() });

    info!("Publisher started");

    // Start NATS
    let docker = clients::Cli::default();
    let (_node, nats_port) = new_nats_docker_container(&docker, Some(free_local_port), false);

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    // Start a subscriber that should receive the message sent when NATS was down
    subscribe_to_nats(
        NatsSubscriberConfig {
            client: NatsClientConfig {
                addresses: vec![format!("127.0.0.1:{}", nats_port)],
                auth: None,
            },
            subject: subject.to_owned(),
        },
        10000,
        move |event| {
            sender.send(event).unwrap();
            Ok(())
        },
    )
    .await
    .unwrap();

    assert_eq!(event, serde_json::from_slice(&receiver.recv().await.unwrap().msg).unwrap());
}

#[actix_rt::test]
#[serial]
async fn subscriber_should_try_reconnect_if_nats_is_not_available_at_startup() {
    start_logger();
    let free_local_port = port_check::free_local_port().unwrap();

    let random: u8 = rand::random();
    let event = Event::new(format!("event_type_{}", random));
    let subject = format!("test_subject_{}", random);

    let subject_clone = subject.clone();
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    actix::spawn(async move {
        // Start a subscriber
        subscribe_to_nats(
            NatsSubscriberConfig {
                client: NatsClientConfig {
                    addresses: vec![format!("127.0.0.1:{}", free_local_port)],
                    auth: None,
                },
                subject: subject_clone,
            },
            10000,
            move |event| {
                sender.send(event).unwrap();
                Ok(())
            },
        )
        .await
        .unwrap();
    });

    time::sleep_until(time::Instant::now() + time::Duration::new(1, 0)).await;

    // Start NATS
    let docker = clients::Cli::default();
    let (_node, nats_port) = new_nats_docker_container(&docker, Some(free_local_port), false);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    // Start a publisher and publish a message when Nats is not available
    let publisher = NatsPublisherActor::start_new(
        NatsPublisherConfig {
            client: NatsClientConfig { addresses: vec![nats_address], auth: None },
            subject: subject.to_owned(),
        },
        10,
    )
    .await
    .unwrap();

    let mut received = false;
    let mut max_attempts: i32 = 30;

    while !received && max_attempts > 0 {
        max_attempts -= 1;
        publisher.do_send(EventMessage { event: event.clone() });
        time::sleep_until(time::Instant::now() + time::Duration::new(1, 0)).await;
        received = receiver.try_recv().is_ok();
        if received {
            info!("Message received by the subscriber");
        } else {
            warn!("Message NOT received by the subscriber... let's retry!");
        }
    }

    assert!(received);
}

#[actix_rt::test]
#[serial]
async fn publisher_and_subscriber_should_reconnect_and_reprocess_events_if_nats_connection_is_lost()
{
    start_logger();
    let free_local_port = port_check::free_local_port().unwrap();

    let random: u8 = rand::random();
    let event = Event::new(format!("event_type_{}", random));
    let subject = format!("test_subject_{}", random);

    let nats_address = format!("127.0.0.1:{}", free_local_port);

    let publisher = NatsPublisherActor::start_new(
        NatsPublisherConfig {
            client: NatsClientConfig { addresses: vec![nats_address.to_owned()], auth: None },
            subject: subject.to_owned(),
        },
        10,
    )
    .await
    .unwrap();

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    actix::spawn(async move {
        subscribe_to_nats(
            NatsSubscriberConfig {
                client: NatsClientConfig { addresses: vec![nats_address.to_owned()], auth: None },
                subject: subject.to_owned(),
            },
            10000,
            move |event| {
                sender.send(event).unwrap();
                Ok(())
            },
        )
        .await
        .unwrap();
    });

    let docker = clients::Cli::default();
    let loops: usize = 3;
    let mut in_flight_messages = 0;
    let mut received_messages = 0;

    for i in 1..=loops {
        let (node, _nats_port) = new_nats_docker_container(&docker, Some(free_local_port), false);

        let mut nats_is_up = true;

        publisher.do_send(EventMessage { event: event.clone() });
        in_flight_messages += 1;
        time::sleep_until(time::Instant::now() + time::Duration::new(1, 0)).await;

        for _ in 0..in_flight_messages {
            assert!(receiver.recv().await.is_some());
            in_flight_messages -= 1;
            received_messages += 1;
        }

        if i != loops {
            drop(node);
            wait_until_port_is_free(free_local_port).await;
            nats_is_up = false;
        };

        publisher.do_send(EventMessage { event: event.clone() });
        in_flight_messages += 1;

        if nats_is_up {
            for _ in 0..in_flight_messages {
                assert!(receiver.recv().await.is_some());
                in_flight_messages -= 1;
                received_messages += 1;
            }
        }
    }

    assert_eq!(loops * 2, received_messages);
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

async fn wait_until_port_is_free(port: u16) {
    while !port_check::is_local_port_free(port) {
        warn!("port {} still not free", port);
        time::sleep_until(time::Instant::now() + time::Duration::new(1, 0)).await;
    }
    time::sleep_until(time::Instant::now() + time::Duration::new(0, 10000)).await;
}
