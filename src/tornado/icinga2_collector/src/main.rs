use crate::actors::tcp_client::EventMessage;
use actix::prelude::*;
use failure::Fail;
use log::*;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common::actors;
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common_logger::setup_logger;

mod actor;
mod config;
mod error;

fn main() -> Result<(), Box<std::error::Error>> {
    let config = config::Conf::build();

    setup_logger(&config.logger).map_err(|err| err.compat())?;

    let streams_dir = format!("{}/{}", &config.io.config_dir, &config.io.streams_dir);
    let streams_config =
        config::read_streams_from_config(&streams_dir).map_err(|err| err.compat())?;

    let icinga2_config_path = format!("{}/{}", &config.io.config_dir, "icinga2_collector.toml");
    let icinga2_config = config::build_icinga2_client_config(&icinga2_config_path)?;

    System::run(move || {
        info!("Starting Icinga2 Collector");

        let tornado_tcp_address = format!(
            "{}:{}",
            config.io.tornado_event_socket_ip, config.io.tornado_event_socket_port
        );
        let tcp_client_addr =
            TcpClientActor::start_new(tornado_tcp_address.clone(), config.io.message_queue_size);

        streams_config.iter().for_each(|config| {
            let config = config.clone();
            let icinga2_config = icinga2_config.clone();
            let tcp_client_addr = tcp_client_addr.clone();
            SyncArbiter::start(1, move || {
                let tcp_client_addr = tcp_client_addr.clone();
                actor::Icinga2StreamActor {
                    icinga_config: icinga2_config.clone(),
                    collector: JMESPathEventCollector::build(config.collector_config.clone())
                        .unwrap_or_else(|e| panic!("Not able to start JMESPath collector with configuration: \n{:#?}. Err: {}", config.collector_config.clone(), e)),
                    stream_config: config.stream.clone(),
                    callback: move |event| {
                        tcp_client_addr.do_send(EventMessage { event });
                    },
                }
            });
        });
    })?;
    Ok(())

}
