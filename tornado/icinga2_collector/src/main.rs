use crate::actors::message::EventMessage;
use actix::prelude::*;
use log::*;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common::actors;
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common_logger::setup_logger;
use actix::dev::ToEnvelope;
use crate::config::{StreamConfig, CollectorConfig};
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common::actors::nats_streaming_publisher::NatsPublisherActor;

mod actor;
mod config;
mod error;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");
    let streams_dir = arg_matches.value_of("streams-dir").expect("streams-dir should be provided");
    let collector_config = config::build_config(&config_dir)?;

    setup_logger(&collector_config.logger)?;

    let streams_dir_full_path = format!("{}/{}", &config_dir, &streams_dir);
    let streams_config = config::read_streams_from_config(&streams_dir_full_path)?;

        info!("Starting Icinga2 Collector");

        match collector_config
            .icinga2_collector
            .tornado_connection_channel
            .unwrap_or(TornadoConnectionChannel::TCP)
        {
            TornadoConnectionChannel::NatsStreaming => {
                info!("Connect to Tornado through NATS Streaming");
                let actor_address = NatsPublisherActor::start_new(
                    collector_config
                        .icinga2_collector
                        .nats
                        .clone()
                        .expect("Nats Streaming config must be provided to connect to a Nats cluster"),
                    collector_config.icinga2_collector.message_queue_size,
                )
                    .await?;
                start(collector_config, streams_config, actor_address);
            }
            TornadoConnectionChannel::TCP => {
                info!("Connect to Tornado through TCP socket");
                // Start TcpWriter
                let tornado_tcp_address = format!(
                    "{}:{}",
                    collector_config.icinga2_collector.tornado_event_socket_ip.clone().expect("'tornado_event_socket_ip' must be provided to connect to a Tornado through TCP"),
                    collector_config.icinga2_collector.tornado_event_socket_port.clone().expect("'tornado_event_socket_port' must be provided to connect to a Tornado through TCP"),
                );

                let actor_address = TcpClientActor::start_new(
                    tornado_tcp_address,
                    collector_config.icinga2_collector.message_queue_size,
                );
                start(collector_config, streams_config, actor_address);
            }
        };

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();

    Ok(())
}

fn start<A: Actor + actix::Handler<EventMessage>>(
    collector_config: CollectorConfig,
    streams_config: Vec<StreamConfig>,
    actor_address: Addr<A>,
) where
    <A as Actor>::Context: ToEnvelope<A, tornado_common::actors::message::EventMessage> {

    streams_config.iter().for_each(|config| {
        let config = config.clone();
        let icinga2_config = collector_config.clone();
        let actor_address = actor_address.clone();
        SyncArbiter::start(1, move || {
            let actor_address = actor_address.clone();
            actor::Icinga2StreamActor {
                icinga_config: icinga2_config.icinga2_collector.connection.clone(),
                collector: JMESPathEventCollector::build(config.collector_config.clone())
                    .unwrap_or_else(|e| panic!("Not able to start JMESPath collector with configuration: \n{:?}. Err: {}", config.collector_config.clone(), e)),
                stream_config: config.stream.clone(),
                callback: move |event| {
                    actor_address.do_send(EventMessage { event });
                },
            }
        });
    });

}