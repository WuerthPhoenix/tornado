use crate::actors::uds_writer::EventMessage;
use actix::prelude::*;
use failure::Fail;
use log::*;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common::actors;
use tornado_common::actors::uds_writer::UdsWriterActor;
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

        // Start UdsWriter
        let uds_writer_addr =
            UdsWriterActor::start_new(config.io.uds_path.clone(), config.io.uds_mailbox_capacity);

        streams_config.iter().for_each(|config| {
            let config = config.clone();
            let icinga2_config = icinga2_config.clone();
            let uds_writer_addr = uds_writer_addr.clone();
            SyncArbiter::start(1, move || {
                let uds_writer_addr = uds_writer_addr.clone();
                actor::Icinga2StreamActor {
                    icinga_config: icinga2_config.clone(),
                    collector: JMESPathEventCollector::build(config.collector_config.clone())
                        .unwrap_or_else(|e| panic!("Not able to start JMESPath collector with configuration: \n{:#?}. Err: {}", config.collector_config.clone(), e)),
                    stream_config: config.stream.clone(),
                    callback: move |event| {
                        uds_writer_addr.do_send(EventMessage { event });
                    },
                }
            });
        });
    });
    Ok(())
}
