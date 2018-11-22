extern crate config as config_rs;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_collector_rsyslog;

#[macro_use]
extern crate log;

pub mod config;

use std::fs;
use std::io;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;
use tornado_common_api::Event;
use tornado_common_logger::setup_logger;

fn main() {
    let conf = config::Conf::new().expect("Should read the configuration");

    // Setup logger
    setup_logger(&conf.logger).unwrap();

    info!("Rsyslog collector started");
    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let mut input = String::new();

    loop {
        match stdin.read_line(&mut input) {
            Ok(len) => if len == 0 {
                return;
            } else {
                trace!("Received line: {}", input);
                input.clear();
            }
            Err(error) => {
                error!("error: {}", error);
                return;
            }
        }
    }

    // Create uds writer
    //let mut stream =
    //    UnixStream::connect(&conf.io.uds_socket_path).expect("Should connect to socket");



    // Send events
    /*
    for _ in 0..conf.io.repeat_send {
        for event in &events {
            write_to_socket(&mut stream, event);
        }
    }
    */

}

fn write_to_socket(stream: &mut UnixStream, event: &Event) {
    let event_bytes = serde_json::to_vec(event).unwrap();
    stream.write_all(&event_bytes).expect("should write event to socket");
    stream.write_all(b"\n").expect("should write endline to socket");
}
