#![cfg(unix)]

pub mod actor;
pub mod config;

use crate::actor::EmailReaderActor;
use actix::{System, Actor, Addr};
use log::*;
use tornado_common::actors::uds_server::listen_to_uds_socket;
use tornado_common_logger::setup_logger;
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common::actors::nats_streaming_publisher::NatsPublisherActor;
use tornado_common::actors::message::EventMessage;
use actix::dev::ToEnvelope;
use tornado_common::actors::tcp_client::TcpClientActor;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let arg_matches = config::arg_matches();

    let collector_config = config::build_config(
        &arg_matches.value_of("config-dir").expect("config-dir should be provided"),
    )?;

    // Setup logger
    setup_logger(&collector_config.logger)?;

    info!("Email collector started");

    match collector_config
        .email_collector
        .tornado_connection_channel
        .unwrap_or(TornadoConnectionChannel::TCP)
    {
        TornadoConnectionChannel::NatsStreaming => {
            info!("Connect to Tornado through NATS Streaming");
            let actor_address = NatsPublisherActor::start_new(
                collector_config
                    .email_collector
                    .nats
                    .expect("Nats Streaming config must be provided to connect to a Nats cluster"),
                collector_config.email_collector.message_queue_size,
            )
                .await?;
            start(collector_config.email_collector.uds_path, actor_address)?;
        }
        TornadoConnectionChannel::TCP => {
            info!("Connect to Tornado through TCP socket");
            // Start TcpWriter
            let tornado_tcp_address = format!(
                "{}:{}",
                collector_config.email_collector.tornado_event_socket_ip.expect("'tornado_event_socket_ip' must be provided to connect to a Tornado through TCP"),
                collector_config.email_collector.tornado_event_socket_port.expect("'tornado_event_socket_port' must be provided to connect to a Tornado through TCP"),
            );

            let actor_address = TcpClientActor::start_new(
                tornado_tcp_address,
                collector_config.email_collector.message_queue_size,
            );
            start(collector_config.email_collector.uds_path, actor_address)?;
        }
    };

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();

    Ok(())
}


fn start<A: Actor + actix::Handler<EventMessage>>(
    uds_path: String,
    actor_address: Addr<A>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
    where
        <A as Actor>::Context: ToEnvelope<A, tornado_common::actors::message::EventMessage> {

    // Start Email collector
    let email_addr = EmailReaderActor::start_new(actor_address);

    // Open UDS socket
    listen_to_uds_socket(
        uds_path.clone(),
        Some(0o770),
        move |msg| {
            email_addr.do_send(msg);
        },
    )
        .and_then(|_| {
            info!(
                "Started UDS server at [{}]. Listening for incoming events",
                uds_path
            );
            Ok(())
        })
        .unwrap_or_else(|err| {
            error!(
                "Cannot start UDS server at [{}]. Err: {}",
                uds_path,
                err
            );
            std::process::exit(1);
        });

    Ok(())
}
