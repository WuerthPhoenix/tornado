#![cfg(unix)]

pub mod actor;
pub mod config;

use crate::actor::EmailReaderActor;
use actix::prelude::*;
use failure::Fail;
use log::*;
use tornado_common::actors::uds_server::listen_to_uds_socket;
use tornado_common_logger::setup_logger;

fn main() -> Result<(), Box<std::error::Error>> {
    let conf = config::Conf::build();

    // Setup logger
    setup_logger(&conf.logger).map_err(|err| err.compat())?;

    info!("Email collector started");

    // start system
    System::run(move || {
        // Start TcpWriter
        let tornado_tcp_address =
            format!("{}:{}", conf.io.tornado_event_socket_ip, conf.io.tornado_event_socket_port);
        let tpc_client_addr = tornado_common::actors::tcp_client::TcpClientActor::start_new(
            tornado_tcp_address.clone(),
            conf.io.message_queue_size,
        );

        // Start Email collector
        let email_addr = EmailReaderActor::start_new(tpc_client_addr.clone());

        // Open UDS socket
        listen_to_uds_socket(conf.io.uds_path.clone(), move |msg| {
            debug!("Received message on the socket");
            email_addr.do_send(msg);
        })
        .and_then(|_| {
            info!(
                "Started UDS server at [{}]. Listening for incoming events",
                conf.io.uds_path.clone()
            );
            Ok(())
        })
        .unwrap_or_else(|err| {
            error!("Cannot start UDS server at [{}]. Err: {}", conf.io.uds_path.clone(), err);
            std::process::exit(1);
        });
    })?;

    Ok(())
}
