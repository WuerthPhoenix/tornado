#![cfg(feature = "nats")]

//
// This tests require docker on the host machine
//

use log::*;
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
) -> (Container<'_, clients::Cli, GenericImage>, String) {
    let port = port.unwrap_or_else(|| port_check::free_local_port().unwrap());

    let mut image = images::generic::GenericImage::new("nats:2.1-alpine");
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
    let node = docker.run_with_options(
        vec!["-d", "-p", &format!("{}:4222", port)],
        image.with_wait_for(images::generic::WaitFor::message_on_stderr("Server is ready")),
    );
    let nats_address = format!("127.0.0.1:{}", node.get_host_port(4222).unwrap());
    (node, nats_address)
}

#[actix_rt::test]
async fn should_publish_to_nats() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_address) = new_nats_docker_container(&docker, None, false);

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
    .unwrap();
    publisher.do_send(EventMessage { event: event.clone() });

    assert_eq!(Some(event), receiver.recv().await);
}

#[actix_rt::test]
async fn should_publish_to_nats_with_tls() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_address) = new_nats_docker_container(&docker, None, true);

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
    .unwrap();
    publisher.do_send(EventMessage { event: event.clone() });

    assert_eq!(Some(event), receiver.recv().await);
}

#[actix_rt::test]
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
    .unwrap();
    publisher.do_send(EventMessage { event: event.clone() });

    info!("Publisher started");

    // Start NATS
    let docker = clients::Cli::default();
    let (_node, nats_address) = new_nats_docker_container(&docker, Some(free_local_port), false);

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    // Start a subscriber that should receive the message sent when NATS was down
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

    assert_eq!(Some(event), receiver.recv().await);
}

#[actix_rt::test]
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

    time::delay_until(time::Instant::now() + time::Duration::new(1, 0)).await;

    // Start NATS
    let docker = clients::Cli::default();
    let (_node, nats_address) = new_nats_docker_container(&docker, Some(free_local_port), false);

    // Start a publisher and publish a message when Nats is not available
    let publisher = NatsPublisherActor::start_new(
        NatsPublisherConfig {
            client: NatsClientConfig { addresses: vec![nats_address], auth: None },
            subject: subject.to_owned(),
        },
        10,
    )
    .unwrap();

    let mut received = false;
    let mut max_attempts: i32 = 30;

    while !received && max_attempts > 0 {
        max_attempts -= 1;
        publisher.do_send(EventMessage { event: event.clone() });
        time::delay_until(time::Instant::now() + time::Duration::new(1, 0)).await;
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
        let (node, _nats_address) =
            new_nats_docker_container(&docker, Some(free_local_port), false);

        let mut nats_is_up = true;

        publisher.do_send(EventMessage { event: event.clone() });
        in_flight_messages += 1;
        time::delay_until(time::Instant::now() + time::Duration::new(1, 0)).await;

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
        time::delay_until(time::Instant::now() + time::Duration::new(1, 0)).await;
    }
    time::delay_until(time::Instant::now() + time::Duration::new(0, 10000)).await;
}
