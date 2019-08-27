#![cfg(unix)]

pub mod actor;
pub mod config;

use crate::actor::EmailReaderActor;
use actix::prelude::*;
use log::*;
use tornado_common::actors::uds_server::listen_to_uds_socket;
use tornado_common_logger::setup_logger;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg_matches = config::arg_matches();

    let collector_config = config::build_config(
        &arg_matches.value_of("config-dir").expect("config-dir should be provided"),
    )?;

    // Setup logger
    setup_logger(&collector_config.logger).map_err(failure::Fail::compat)?;

    info!("Email collector started");

    // start system
    System::run(move || {
        // Start TcpWriter
        let tornado_tcp_address = format!(
            "{}:{}",
            collector_config.email_collector.tornado_event_socket_ip,
            collector_config.email_collector.tornado_event_socket_port
        );
        let tpc_client_addr = tornado_common::actors::tcp_client::TcpClientActor::start_new(
            tornado_tcp_address.clone(),
            collector_config.email_collector.message_queue_size,
        );

        // Start Email collector
        let email_addr = EmailReaderActor::start_new(tpc_client_addr.clone());

        // Open UDS socket
        listen_to_uds_socket(
            collector_config.email_collector.uds_path.clone(),
            Some(0o770),
            move |msg| {
                debug!("Received message on the socket");
                email_addr.do_send(msg);
            },
        )
        .and_then(|_| {
            info!(
                "Started UDS server at [{}]. Listening for incoming events",
                collector_config.email_collector.uds_path.clone()
            );
            Ok(())
        })
        .unwrap_or_else(|err| {
            error!(
                "Cannot start UDS server at [{}]. Err: {}",
                collector_config.email_collector.uds_path.clone(),
                err
            );
            std::process::exit(1);
        });
    })?;

    Ok(())
}
