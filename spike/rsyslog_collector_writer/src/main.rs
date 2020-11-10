pub mod config;

use log::*;
use std::fs;
use std::io;
use std::{thread, time};
use tornado_common_api::{Payload, Value};
use tornado_common_logger::{setup_logger, LoggerConfig};

fn main() {
    let conf = config::Conf::build();

    let logger_config =
        LoggerConfig { level: "Debug".to_owned(), stdout_output: true, file_output_path: None };

    // Setup logger
    let _guard = setup_logger(&logger_config).unwrap();

    // Load events from fs
    let events = read_events_from_config(&conf.io.json_events_path);

    // Create uds writer
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Send events
    let sleep_millis = time::Duration::from_millis(conf.io.repeat_sleep_ms);

    for i in 0..conf.io.repeat_send {
        for event in &events {
            let mut event_clone = event.clone();
            event_clone.insert("count".to_owned(), Value::Text(i.to_string()));
            write(&mut handle, &event_clone);
            thread::sleep(sleep_millis);
        }
    }

    info!("Completed sending {} events", conf.io.repeat_send * events.len());
}

fn read_events_from_config(path: &str) -> Vec<Payload> {
    let paths = fs::read_dir(path).unwrap();
    let mut events = vec![];

    for path in paths {
        let filename = path.unwrap().path();
        let event_body = fs::read_to_string(&filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename.display()));
        events.push(serde_json::from_str(&event_body).unwrap());
    }

    info!("Loaded {} event(s) from [{}]", events.len(), path);

    events
}

fn write(stdout: &mut dyn io::Write, event: &Payload) {
    let event_bytes = serde_json::to_vec(event).unwrap();
    stdout.write_all(&event_bytes).expect("should write event to socket");
    stdout.write_all(b"\n").expect("should write endline to socket");
}
