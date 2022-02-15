#![cfg(feature = "nats_integration_tests")]

//
// This tests require docker on the host machine
//

use std::time::Duration;

use actix::clock::sleep;
use log::*;
use reqwest::Client;
use serde_json::{Map, Number, Value};
use serial_test::serial;
use std::sync::Arc;
use testcontainers::images::generic::GenericImage;
use testcontainers::*;
use tokio::time;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::nats_publisher::{
    NatsClientAuth, NatsClientConfig, NatsPublisherActor, NatsPublisherConfig,
};
use tornado_common::actors::nats_subscriber::{subscribe_to_nats, NatsSubscriberConfig};
use tornado_common_api::{Event, TracedEvent};
use tornado_common_logger::elastic_apm::ApmTracingConfig;
use tracing::Span;

fn new_nats_docker_container(
    docker: &clients::Cli,
    port: Option<u16>,
    tls: bool,
) -> (Container<'_, clients::Cli, GenericImage>, u16, u16) {
    let port = port.unwrap_or_else(|| port_check::free_local_port().unwrap());
    let monitoring_port = port_check::free_local_port().unwrap();
    let internal_monitoring_port = 8222;

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
                "--http_port".to_owned(),
                internal_monitoring_port.to_string(),
            ]);
    } else {
        image = image.with_args(vec![
            "nats-server".to_owned(),
            "--http_port".to_owned(),
            internal_monitoring_port.to_string(),
        ]);
    }

    let args = RunArgs::default()
        .with_mapped_port((port, 4222))
        .with_mapped_port((monitoring_port, internal_monitoring_port));
    let node = docker.run_with_args(
        image.with_wait_for(images::generic::WaitFor::message_on_stderr("Server is ready")),
        args,
    );

    let nats_port = node.get_host_port(4222).unwrap();
    (node, nats_port, monitoring_port)
}

async fn get_number_of_published_messages(nats_monitoring_url: &str) -> u64 {
    let connections = get_nats_connections_status(nats_monitoring_url).await;

    match connections.get("connections").unwrap().get(0) {
        None => 0,
        Some(connection_stats) => connection_stats.get("in_msgs").unwrap().as_u64().unwrap(),
    }
}

async fn get_nats_connections_status(nats_monitoring_url: &str) -> Value {
    Client::new()
        .get(format!("http://{}/connz", nats_monitoring_url))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()
}

#[actix_rt::test]
#[serial]
async fn official_nats_client_spike_test() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_port, _nats_monitoring_port) = new_nats_docker_container(&docker, None, false);
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
    let (_node, nats_port, _nats_monitoring_port) = new_nats_docker_container(&docker, None, false);
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

    assert_eq!(serde_json::to_vec(&event).unwrap(), receiver.recv().await.unwrap().msg.data);
}

#[actix_rt::test]
#[serial]
async fn nats_publisher_should_publish_to_nats() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_port, _nats_monitoring_port) = new_nats_docker_container(&docker, None, false);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    let random: u8 = rand::random();
    let subject = format!("test_subject_{}", random);
    let mut event = Event::new(format!("event_type_{}", random));
    let mut metadata = Map::new();
    metadata.insert(
        "some_metadata".to_owned(),
        Value::Array(vec![Value::String("val1".to_owned()), Value::String("val2".to_owned())]),
    );
    event.metadata = metadata;

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
    publisher.do_send(EventMessage(TracedEvent { event: event.clone(), span: Span::current() }));

    let mut received_event: Event =
        serde_json::from_slice(receiver.recv().await.unwrap().as_slice()).unwrap();
    received_event.metadata.remove("trace_context");
    assert_eq!(event, received_event);
}

#[actix_rt::test]
#[serial]
async fn should_publish_to_nats() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_port, _nats_monitoring_port) = new_nats_docker_container(&docker, None, false);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    let random: u8 = rand::random();
    let mut event = Event::new(format!("event_type_{}", random));
    let mut metadata = Map::new();
    metadata.insert("some_metadata".to_owned(), Value::Number(Number::from(1)));
    event.metadata = metadata;
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
    publisher.do_send(EventMessage(TracedEvent { event: event.clone(), span: Span::current() }));

    let mut received: Event =
        serde_json::from_slice(&receiver.recv().await.unwrap().msg.data).unwrap();
    // We don't want to test the trace_context since it is added by the NatsPublisherActor
    received.metadata.remove("trace_context");
    assert_eq!(event, received);
}

#[actix_rt::test]
#[serial]
async fn should_publish_to_nats_with_tls() {
    start_logger();
    let docker = clients::Cli::default();
    let (_node, nats_port, _nats_monitoring_port) = new_nats_docker_container(&docker, None, true);
    let nats_address = format!("localhost:{}", nats_port);

    let random: u8 = rand::random();
    let mut event = Event::new(format!("event_type_{}", random));
    let mut metadata = Map::new();
    metadata.insert("some_metadata".to_owned(), Value::Number(Number::from(1)));
    event.metadata = metadata;
    let subject = format!("test_subject_{}", random);

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    let auth = Some(NatsClientAuth::Tls {
        certificate_path: "./test_resources/nats-client.crt.pem".to_string(),
        path_to_root_certificate: Some("./test_resources/root-ca.crt".to_string()),
        private_key_path: "./test_resources/nats-client.key.pem".to_string(),
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
    publisher.do_send(EventMessage(TracedEvent { event: event.clone(), span: Span::current() }));

    let mut received: Event =
        serde_json::from_slice(&receiver.recv().await.unwrap().msg.data).unwrap();
    // We don't want to test the trace_context since it is added by the NatsPublisherActor
    received.metadata.remove("trace_context");
    assert_eq!(event, received);
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
    publisher.do_send(EventMessage(TracedEvent { event: event.clone(), span: Span::current() }));

    info!("Publisher started");

    // Start NATS
    let docker = clients::Cli::default();
    let (_node, _nats_port, nats_monitoring_port) =
        new_nats_docker_container(&docker, Some(free_local_port), false);

    let mut published_messages = 0;
    while published_messages < 1 {
        published_messages =
            get_number_of_published_messages(&format!("127.0.0.1:{}", nats_monitoring_port)).await;
        time::sleep(Duration::from_secs(1)).await;
    }

    assert_eq!(published_messages, 1);
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
    let (_node, nats_port, nats_monitoring_port) =
        new_nats_docker_container(&docker, Some(free_local_port), false);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    while get_nats_connections_status(&format!("127.0.0.1:{}", nats_monitoring_port))
        .await
        .get("num_connections")
        .unwrap()
        .as_u64()
        .unwrap()
        < 1
    {
        info!("Wait for subscriber to be connected to NATS");
        time::sleep(Duration::from_secs(1)).await;
    }

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
        publisher
            .do_send(EventMessage(TracedEvent { event: event.clone(), span: Span::current() }));
        time::sleep_until(time::Instant::now() + time::Duration::new(1, 0)).await;
        received = receiver.recv().await.is_some();
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

    let subscriber_connected = Arc::new(std::sync::RwLock::new(false));
    let subscriber_connected_clone = subscriber_connected.clone();
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
        let mut lock = subscriber_connected_clone.write().unwrap();
        *lock = true;
    });

    let docker = clients::Cli::default();
    let loops: usize = 3;
    let mut in_flight_messages = 0;
    let mut received_messages = 0;

    for i in 1..=loops {
        let (node, _nats_port, _nats_monitoring_port) =
            new_nats_docker_container(&docker, Some(free_local_port), false);

        let mut nats_is_up = true;

        // Before publishing the event, wait the subscriber to be subscribed for the first time.
        // The subscriber may be slower than the publisher to connect to NATS for the first time.
        // If this is the case, the message could be successfully published before the subscriber
        // is subscribed. The message would then be not received.
        loop {
            if *(subscriber_connected.read().unwrap()) {
                publisher.do_send(EventMessage(TracedEvent {
                    event: event.clone(),
                    span: Span::current(),
                }));
                break;
            }
            info!("Subscriber not yet connected, delaying publishing");
            sleep(time::Duration::from_secs(1)).await;
        }

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

        publisher
            .do_send(EventMessage(TracedEvent { event: event.clone(), span: Span::current() }));
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

// Tests that all event received by the publisher after a disconnection from NATS, are correctly
// published. See https://github.com/nats-io/nats.rs/issues/182
#[actix_rt::test]
#[serial]
async fn publisher_should_reschedule_all_events_after_a_disconnection() {
    start_logger();
    let free_local_port = port_check::free_local_port().unwrap();

    // Start NATS
    let docker = clients::Cli::default();
    let (node, nats_port, _nats_monitoring_port) =
        new_nats_docker_container(&docker, Some(free_local_port), false);
    let nats_address = format!("127.0.0.1:{}", nats_port);

    let random: u8 = rand::random();
    let event = Event::new(format!("event_type_{}", random));
    let subject = format!("test_subject_{}", random);

    let publisher = NatsPublisherActor::start_new(
        NatsPublisherConfig {
            client: NatsClientConfig { addresses: vec![nats_address], auth: None },
            subject: subject.to_owned(),
        },
        10,
    )
    .await
    .unwrap();

    time::sleep(time::Duration::new(1, 0)).await;

    drop(node);
    wait_until_port_is_free(free_local_port).await;

    let n_events = 10;

    for i in 0..n_events {
        info!("Sending event to publisher: {}", i);
        publisher
            .do_send(EventMessage(TracedEvent { event: event.clone(), span: Span::current() }));
        time::sleep(time::Duration::from_millis(100)).await;
    }

    let (_node, _nats_port, nats_monitoring_port) =
        new_nats_docker_container(&docker, Some(free_local_port), false);

    let mut n_published;
    let mut retries = 5;
    loop {
        n_published =
            get_number_of_published_messages(&format!("127.0.0.1:{}", nats_monitoring_port)).await;
        info!("Subscriber received {} messages", n_published);
        if n_published < n_events {
            time::sleep(Duration::from_secs(1)).await;
        } else {
            break;
        }
        retries = retries - 1;
        if retries <= 0 {
            break;
        }
    }
    assert_eq!(n_published, n_events);
}

fn start_logger() {
    println!("Init logger");

    let conf = tornado_common_logger::LoggerConfig {
        level: String::from("trace"),
        stdout_output: true,
        file_output_path: None,
        tracing_elastic_apm: ApmTracingConfig {
            apm_output: false,
            apm_server_url: "http://localhost:8200".to_owned(),
            apm_server_api_credentials: None,
            exporter: Default::default(),
        },
    };
    if let Err(err) = tornado_common_logger::setup_logger(conf) {
        println!("Warn: err starting logger: {:?}", err)
    };
}

async fn wait_until_port_is_free(port: u16) {
    while !port_check::is_local_port_free(port) {
        warn!("port {} still not free", port);
        time::sleep_until(time::Instant::now() + time::Duration::new(1, 0)).await;
    }
    time::sleep_until(time::Instant::now() + time::Duration::new(0, 10000)).await;
}
