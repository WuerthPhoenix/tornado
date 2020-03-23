use crate::actors::message::EventMessage;
use crate::config::{CollectorConfig, StreamConfig};
use actix::dev::ToEnvelope;
use actix::prelude::*;
use log::*;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common::actors::nats_publisher::NatsPublisherActor;
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common::{actors, TornadoError};
use tornado_common_logger::setup_logger;

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

    //
    // WARN:
    // This 'if' block contains some duplicated code to allow temporary compatibility with the config file format of the previous release.
    // It will be removed in the next release when the `tornado_connection_channel` will be mandatory.
    //
    if let (Some(tornado_event_socket_ip), Some(tornado_event_socket_port)) = (
        collector_config.icinga2_collector.tornado_event_socket_ip.as_ref(),
        collector_config.icinga2_collector.tornado_event_socket_port.as_ref(),
    ) {
        info!("Connect to Tornado through TCP socket");
        // Start TcpWriter
        let tornado_tcp_address =
            format!("{}:{}", tornado_event_socket_ip, tornado_event_socket_port,);

        let actor_address = TcpClientActor::start_new(
            tornado_tcp_address,
            collector_config.icinga2_collector.message_queue_size,
        );
        start(collector_config, streams_config, actor_address);
    } else if let Some(connection_channel) =
        &collector_config.icinga2_collector.tornado_connection_channel
    {
        match connection_channel {
            TornadoConnectionChannel::NatsStreaming { nats_streaming } => {
                info!("Connect to Tornado through NATS Streaming");
                let actor_address = NatsPublisherActor::start_new(
                    nats_streaming,
                    collector_config.icinga2_collector.message_queue_size,
                )
                .await?;
                start(collector_config, streams_config, actor_address);
            }
            TornadoConnectionChannel::TCP { tcp_socket_ip, tcp_socket_port } => {
                info!("Connect to Tornado through TCP socket");
                // Start TcpWriter
                let tornado_tcp_address = format!("{}:{}", tcp_socket_ip, tcp_socket_port,);

                let actor_address = TcpClientActor::start_new(
                    tornado_tcp_address,
                    collector_config.icinga2_collector.message_queue_size,
                );
                start(collector_config, streams_config, actor_address);
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
    collector_config: CollectorConfig,
    streams_config: Vec<StreamConfig>,
    actor_address: Addr<A>,
) where
    <A as Actor>::Context: ToEnvelope<A, tornado_common::actors::message::EventMessage>,
{
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
