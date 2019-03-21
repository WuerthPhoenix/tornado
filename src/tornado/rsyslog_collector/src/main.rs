pub mod actors;
pub mod config;

use actix::prelude::*;
use failure::Fail;
use log::*;
use std::io::{stdin, BufRead};
use std::thread;
use tornado_common_logger::setup_logger;

fn main() -> Result<(), Box<std::error::Error>> {
    let conf = config::Conf::build();

    // Setup logger
    setup_logger(&conf.logger).map_err(|err| err.compat())?;

    info!("Rsyslog collector started");

    // start system
    System::run(move || {
        // Start UdsWriter
        let uds_writer_addr = tornado_common::actors::uds_client::UdsClientActor::start_new(
            conf.io.uds_path.clone(),
            conf.io.uds_mailbox_capacity,
        );

        // Start Rsyslog collector
        // actors::collector::RsyslogCollectorActor::start_new(tokio::io::stdin(), uds_writer_addr.clone());

        // Start Rsyslog collector
        let rsyslog_addr = SyncArbiter::start(1, move || {
            actors::sync_collector::RsyslogCollectorActor::new(uds_writer_addr.clone())
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
                            rsyslog_addr
                                .do_send(actors::sync_collector::RsyslogMessage { json: input });
                        }
                    }
                    Err(error) => {
                        error!("error: {}", error);
                        system.stop();
                    }
                }
            }
        });
    });

    Ok(())
}
