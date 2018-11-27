extern crate actix;
extern crate config as config_rs;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_io;
extern crate tokio_uds;
extern crate tornado_collector_common;
extern crate tornado_collector_rsyslog;
extern crate tornado_common_api;
extern crate tornado_common_logger;

#[macro_use]
extern crate log;

pub mod actors;
pub mod config;

use actix::prelude::*;
use tornado_common_logger::setup_logger;

fn main() {
    let conf = config::Conf::new().expect("Should read the configuration");

    // Setup logger
    setup_logger(&conf.logger).unwrap();

    info!("Rsyslog collector started");

    // start system
    System::run(move || {

        // Start UdsWriter
        let uds_writer_addr = actors::uds_writer::UdsWriterActor::start_new(
            conf.io.uds_socket_path.clone(),
            conf.io.uds_socket_mailbox_capacity,
        );

        // Start Rsyslog collector
        let stdin = tokio::io::stdin();
        actors::collector::RsyslogCollectorActor::start_new(stdin, uds_writer_addr);

    });
}
