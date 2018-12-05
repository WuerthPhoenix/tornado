extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate structopt;
extern crate tornado_common_api;
extern crate tornado_common_logger;

#[macro_use]
extern crate log;

pub mod config;

use std::fs;
use std::io::prelude::*;
use std::os::unix::net::UnixStream;
use tornado_common_api::Event;
use tornado_common_logger::setup_logger;

fn main() {
    let conf = config::Conf::build();

    // Setup logger
    setup_logger(&conf.logger).unwrap();

    // Load events from fs
    let events = read_events_from_config(&conf.io.json_events_path);

    // Create uds writer
    let mut stream = UnixStream::connect(&conf.io.uds_path).expect("Should connect to socket");

    // Send events
    for _ in 0..conf.io.repeat_send {
        for event in &events {
            write_to_socket(&mut stream, event);
        }
    }

    info!("Completed sending {} events", conf.io.repeat_send * events.len());
}

fn read_events_from_config(path: &str) -> Vec<Event> {
    let paths = fs::read_dir(path).unwrap();
    let mut events = vec![];

    for path in paths {
        let filename = path.unwrap().path();
        info!("Loading event from file: [{}]", filename.display());
        let event_body = fs::read_to_string(&filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename.display()));
        trace!("Event body: \n{}", event_body);
        events.push(serde_json::from_str(&event_body).unwrap());
    }

    info!("Loaded {} event(s) from [{}]", events.len(), path);

    events
}

fn write_to_socket(stream: &mut UnixStream, event: &Event) {
    debug!("Sending event: \n{:?}", event);
    let event_bytes = serde_json::to_vec(event).unwrap();
    stream.write_all(&event_bytes).expect("should write event to socket");
    stream.write_all(b"\n").expect("should write endline to socket");
}
