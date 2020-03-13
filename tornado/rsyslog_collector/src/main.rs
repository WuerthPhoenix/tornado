pub mod actors;
pub mod config;

use actix::dev::ToEnvelope;
use actix::prelude::*;
use log::*;
use std::io::{stdin, BufRead};
use std::thread;
use tornado_common::actors::message::{EventMessage, StringMessage};
use tornado_common::actors::nats_streaming_publisher::NatsPublisherActor;
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common_logger::setup_logger;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");

    let collector_config = config::build_config(&config_dir)?;

    // Setup logger
    setup_logger(&collector_config.logger)?;

    info!("Rsyslog collector started");

    match collector_config
        .rsyslog_collector
        .tornado_connection_channel
        .unwrap_or(TornadoConnectionChannel::TCP)
    {
        TornadoConnectionChannel::NatsStreaming => {
            info!("Connect to Tornado through NATS Streaming");
            let actor_address = NatsPublisherActor::start_new(
                collector_config
                    .rsyslog_collector
                    .nats
                    .expect("Nats Streaming config must be provided to connect to a Nats cluster"),
                collector_config.rsyslog_collector.message_queue_size,
            )
            .await?;
            start(actor_address)?;
        }
        TornadoConnectionChannel::TCP => {
            info!("Connect to Tornado through TCP socket");
            // Start TcpWriter
            let tornado_tcp_address = format!(
                "{}:{}",
                collector_config.rsyslog_collector.tornado_event_socket_ip.expect("'tornado_event_socket_ip' must be provided to connect to a Tornado through TCP"),
                collector_config.rsyslog_collector.tornado_event_socket_port.expect("'tornado_event_socket_port' must be provided to connect to a Tornado through TCP"),
            );

            let actor_address = TcpClientActor::start_new(
                tornado_tcp_address,
                collector_config.rsyslog_collector.message_queue_size,
            );
            start(actor_address)?;
        }
    };

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();

    Ok(())
}

fn start<A: Actor + actix::Handler<EventMessage>>(
    actor_address: Addr<A>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    <A as Actor>::Context: ToEnvelope<A, tornado_common::actors::message::EventMessage>,
{
    // Start Rsyslog collector
    let rsyslog_addr = SyncArbiter::start(1, move || {
        actors::sync_collector::RsyslogCollectorActor::new(actor_address.clone())
    });

    let system = System::current();
    thread::spawn(move || {
        let stdin = stdin();
        let mut stdin_lock = stdin.lock();

        loop {
            let mut input = String::new();
            match stdin_lock.read_line(&mut input) {
                Ok(len) => {
                    if len == 0 {
                        info!("EOF received. Stopping Rsyslog collector.");
                        system.stop();
                    } else {
                        rsyslog_addr.do_send(StringMessage { msg: input });
                    }
                }
                Err(error) => {
                    error!("error: {}", error);
                    system.stop();
                }
            }
        }
    });

    Ok(())
}
