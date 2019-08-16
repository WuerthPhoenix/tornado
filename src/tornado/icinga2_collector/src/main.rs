use crate::actors::tcp_client::EventMessage;
use actix::prelude::*;
use log::*;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common::actors;
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common_logger::setup_logger;

mod actor;
mod config;
mod error;

fn main() -> Result<(), Box<std::error::Error>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");
    let streams_dir = arg_matches.value_of("streams-dir").expect("streams-dir should be provided");
    let icinga2_config = config::build_config(&config_dir)?;

    setup_logger(&icinga2_config.logger).map_err(failure::Fail::compat)?;

    let streams_dir_full_path = format!("{}/{}", &config_dir, &streams_dir);
    let streams_config =
        config::read_streams_from_config(&streams_dir_full_path).map_err(failure::Fail::compat)?;

    System::run(move || {
        info!("Starting Icinga2 Collector");

        let tornado_tcp_address = format!(
            "{}:{}",
            icinga2_config.icinga2_collector.tornado_event_socket_ip,
            icinga2_config.icinga2_collector.tornado_event_socket_port
        );
        let tcp_client_addr = TcpClientActor::start_new(
            tornado_tcp_address.clone(),
            icinga2_config.icinga2_collector.message_queue_size,
        );

        streams_config.iter().for_each(|config| {
            let config = config.clone();
            let icinga2_config = icinga2_config.clone();
            let tcp_client_addr = tcp_client_addr.clone();
            SyncArbiter::start(1, move || {
                let tcp_client_addr = tcp_client_addr.clone();
                actor::Icinga2StreamActor {
                    icinga_config: icinga2_config.icinga2_collector.connection.clone(),
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
