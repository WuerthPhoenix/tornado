extern crate serde_json;
extern crate tornado_collector_common;
extern crate tornado_collector_json;
extern crate tornado_common_api;
extern crate tornado_common_logger;

#[macro_use]
extern crate log;

use std::collections::HashMap;
use std::io::prelude::*;
use std::fs;
use std::os::unix::net::UnixStream;
use tornado_common_api::Event;
use tornado_common_logger::{setup_logger, LoggerConfig};

fn main() {
    // Setup logger
    let mut conf = LoggerConfig {
        root_level: String::from("info"),
        output_system_enabled: true,
        output_file_enabled: false,
        output_file_name: String::from(""),
        module_level: HashMap::new(),
    };

    conf.module_level.insert("uds_writer_collector".to_owned(), "debug".to_owned());

    setup_logger(&conf).unwrap();

    // Load events from fs
    let config_path = "./config";
    let config_events_path = format!("{}{}", config_path, "/events");
    let events = read_events_from_config(&config_events_path);

    // Create uds writer
    let sock_path = "/tmp/something";
    let mut stream = UnixStream::connect(sock_path).expect("Should connect to socket");

    // Send events
    for event in events {
        write_to_socket(&mut stream, &event);
    }

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
    let event_bytes = serde_json::to_vec(event).unwrap();
    stream.write_all(&event_bytes).expect("should write event to socket");
    stream.write_all(b"\n").expect("should write endline to socket");
}
