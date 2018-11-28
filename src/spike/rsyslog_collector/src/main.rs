extern crate config as config_rs;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tornado_collector_common;
extern crate tornado_collector_json;
extern crate tornado_common_api;
extern crate tornado_common_logger;

#[macro_use]
extern crate log;

pub mod config;

use std::io;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;
use tornado_collector_common::Collector;
use tornado_common_api::Event;
use tornado_common_logger::setup_logger;

fn main() {
    let conf = config::Conf::new().expect("Should read the configuration");

    // Setup logger
    setup_logger(&conf.logger).unwrap();

    info!("Rsyslog collector started");

    // Create uds writer
    let mut stream = UnixStream::connect(&conf.io.uds_socket_path)
        .unwrap_or_else(|_| panic!("Cannot connect to socket on [{}]", &conf.io.uds_socket_path));

    // Create rsyslog collector
    let collector = tornado_collector_json::JsonPayloadCollector::new("syslog");

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();

    let mut input = String::new();

    loop {
        match stdin_lock.read_line(&mut input) {
            Ok(len) => if len == 0 {
                info!("EOF received. Stopping Rsyslog collector.");
                return;
            } else {
                info!("Received line: {}", input);
                let event = collector.to_event(&input).unwrap();
                write_to_socket(&mut stream, &event);
                input.clear();
            },
            Err(error) => {
                error!("error: {}", error);
                return;
            }
        }
    }
}

fn write_to_socket(stream: &mut UnixStream, event: &Event) {
    let event_bytes = serde_json::to_vec(event).unwrap();
    stream.write_all(&event_bytes).expect("should write event to socket");
    stream.write_all(b"\n").expect("should write endline to socket");
}
