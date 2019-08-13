pub mod actors;
pub mod config;

use actix::prelude::*;
use log::*;
use std::io::{stdin, BufRead};
use std::thread;
use tornado_common::actors::message::StringMessage;
use tornado_common_logger::setup_logger;

fn main() -> Result<(), Box<std::error::Error>> {
    let config = config::Conf::build();

    let collector_config_path = format!("{}/{}", &config.io.config_dir, "rsyslog_collector.toml");
    let collector_config = config::build_config(&collector_config_path)?;

    // Setup logger
    setup_logger(&collector_config.logger).map_err(failure::Fail::compat)?;

    info!("Rsyslog collector started");

    // start system
    System::run(move || {
        // Start UdsWriter
        let tornado_tcp_address = format!(
            "{}:{}",
            config.io.tornado_event_socket_ip, config.io.tornado_event_socket_port
        );
        let tpc_client_addr = tornado_common::actors::tcp_client::TcpClientActor::start_new(
            tornado_tcp_address.clone(),
            config.io.message_queue_size,
        );

        // Start Rsyslog collector
        // actors::collector::RsyslogCollectorActor::start_new(tokio::io::stdin(), tpc_client_addr.clone());

        // Start Rsyslog collector
        let rsyslog_addr = SyncArbiter::start(1, move || {
            actors::sync_collector::RsyslogCollectorActor::new(tpc_client_addr.clone())
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
                            debug!("Received line: {}", input);
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
    })?;

    Ok(())
}
