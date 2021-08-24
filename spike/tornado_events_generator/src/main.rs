pub mod config;

use log::*;
use std::fs;
use std::io::prelude::*;
use std::net::TcpStream;
use std::{thread, time};
use tornado_common_api::Event;
use tornado_common_logger::{setup_logger, LoggerConfig};

fn main() {
    let conf = config::Conf::build();

    let logger_config = LoggerConfig {
        level: "Debug".to_owned(),
        stdout_output: true,
        file_output_path: None,
        tracing_elastic_apm: None,
    };

    // Setup logger
    let _guard = setup_logger(logger_config).unwrap();

    // Load events from fs
    let events_path = format!("{}/{}", conf.io.config_dir, conf.io.events_dir);
    let events = read_events_from_config(&events_path);

    // Create uds writer
    let mut stream =
        TcpStream::connect(&conf.io.tornado_tcp_address).expect("Should connect to TCP socket");

    // Send events
    let sleep_millis = time::Duration::from_millis(conf.io.repeat_sleep_ms);

    for _ in 0..conf.io.repeat_send {
        for event in &events {
            write_to_socket(&mut stream, event);
            thread::sleep(sleep_millis);
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

fn write_to_socket(stream: &mut TcpStream, event: &Event) {
    debug!("Sending event: \n{:?}", event);
    let event_bytes = serde_json::to_vec(event).unwrap();
    stream.write_all(&event_bytes).expect("should write event to socket");
    stream.write_all(b"\n").expect("should write endline to socket");
}
