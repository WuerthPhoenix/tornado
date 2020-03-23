#![cfg(unix)]

pub mod actor;
pub mod config;

use crate::actor::EmailReaderActor;
use actix::dev::ToEnvelope;
use actix::{Actor, Addr, System};
use log::*;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::nats_publisher::NatsPublisherActor;
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common::actors::uds_server::listen_to_uds_socket;
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common::TornadoError;
use tornado_common_logger::setup_logger;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let arg_matches = config::arg_matches();

    let collector_config = config::build_config(
        &arg_matches.value_of("config-dir").expect("config-dir should be provided"),
    )?;

    // Setup logger
    setup_logger(&collector_config.logger)?;

    info!("Email collector started");

    //
    // WARN:
    // This 'if' block contains some duplicated code to allow temporary compatibility with the config file format of the previous release.
    // It will be removed in the next release when the `tornado_connection_channel` will be mandatory.
    //
    if let (Some(tornado_event_socket_ip), Some(tornado_event_socket_port)) = (
        collector_config.email_collector.tornado_event_socket_ip,
        collector_config.email_collector.tornado_event_socket_port,
    ) {
        info!("Connect to Tornado through TCP socket");
        // Start TcpWriter
        let tornado_tcp_address =
            format!("{}:{}", tornado_event_socket_ip, tornado_event_socket_port,);

        let actor_address = TcpClientActor::start_new(
            tornado_tcp_address,
            collector_config.email_collector.message_queue_size,
        );
        start(collector_config.email_collector.uds_path, actor_address)?;
    } else if let Some(connection_channel) =
        collector_config.email_collector.tornado_connection_channel
    {
        match connection_channel {
            TornadoConnectionChannel::NatsStreaming { nats_streaming } => {
                info!("Connect to Tornado through NATS Streaming");
                let actor_address = NatsPublisherActor::start_new(
                    &nats_streaming,
                    collector_config.email_collector.message_queue_size,
                )
                .await?;
                start(collector_config.email_collector.uds_path, actor_address)?;
            }
            TornadoConnectionChannel::TCP { tcp_socket_ip, tcp_socket_port } => {
                info!("Connect to Tornado through TCP socket");
                // Start TcpWriter
                let tornado_tcp_address = format!("{}:{}", tcp_socket_ip, tcp_socket_port,);

                let actor_address = TcpClientActor::start_new(
                    tornado_tcp_address,
                    collector_config.email_collector.message_queue_size,
                );
                start(collector_config.email_collector.uds_path, actor_address)?;
            }
        };
    } else {
        return Err(TornadoError::ConfigurationError {
            message: "A communication channel must be specified.".to_owned(),
        }
        .into());
    }

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
    <A as Actor>::Context: ToEnvelope<A, tornado_common::actors::message::EventMessage>,
{
    // Start Email collector
    let email_addr = EmailReaderActor::start_new(actor_address);

    // Open UDS socket
    listen_to_uds_socket(uds_path.clone(), Some(0o770), move |msg| {
        email_addr.do_send(msg);
    })
    .and_then(|_| {
        info!("Started UDS server at [{}]. Listening for incoming events", uds_path);
        Ok(())
    })
    .unwrap_or_else(|err| {
        error!("Cannot start UDS server at [{}]. Err: {}", uds_path, err);
        std::process::exit(1);
    });

    Ok(())
}
