extern crate tornado_collector_common;
extern crate tornado_collector_json;
extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_engine_matcher;
extern crate tornado_executor_common;
extern crate tornado_executor_logger;
extern crate tornado_network_common;
extern crate tornado_network_simple;

extern crate actix;
extern crate bytes;
extern crate futures;
#[macro_use]
extern crate log;
extern crate num_cpus;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_uds;

pub mod collector;
pub mod engine;
pub mod executor;
pub mod reader;

use actix::prelude::*;
use collector::JsonReaderActor;
use engine::MatcherActor;
use executor::ExecutorActor;
use reader::uds::listen_to_uds_socket;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tornado_common_logger::{setup_logger, LoggerConfig};
use tornado_engine_matcher::config::Rule;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tornado_engine_matcher::matcher::Matcher;
use tornado_network_simple::SimpleEventBus;

fn main() {
    // Setup logger
    let mut conf = LoggerConfig {
        root_level: String::from("info"),
        output_system_enabled: true,
        output_file_enabled: false,
        output_file_name: String::from(""),
        module_level: HashMap::new(),
    };

    conf.module_level.insert("tornado_spike_actix".to_owned(), "debug".to_owned());

    setup_logger(&conf).unwrap();

    // Load rules from fs
    let config_path = "./config";
    let config_rules_path = format!("{}{}", config_path, "/rules");
    let config_rules = read_rules_from_config(&config_rules_path);

    // Start matcher & dispatcher
    let matcher = Arc::new(Matcher::new(&config_rules).unwrap());
    //let event_bus = Arc::new(SimpleEventBus::new());
    //let dispatcher = Arc::new(Dispatcher::new(event_bus.clone()).unwrap());

    // start system
    System::run(|| {
        let cpus = num_cpus::get();
        info!("Available CPUs: {}", cpus);

        // Start executor
        let executor_actor = SyncArbiter::start(1, move || {
            let event_bus = Arc::new(SimpleEventBus::new());
            let dispatcher = Dispatcher::new(event_bus.clone()).unwrap();
            ExecutorActor { dispatcher }
        });

        // Start engine
        let matcher_actor = SyncArbiter::start(cpus, move || MatcherActor {
            matcher: matcher.clone(),
            executor_addr: executor_actor.clone(),
        });

        // Start collector
        let sock_path = "/tmp/something";
        listen_to_uds_socket(sock_path, move |msg| {
            JsonReaderActor::start_new(msg, matcher_actor.clone());
        });
    });
}

fn read_rules_from_config(path: &str) -> Vec<Rule> {
    let paths = fs::read_dir(path).unwrap();
    let mut rules = vec![];

    for path in paths {
        let filename = path.unwrap().path();
        info!("Loading rule from file: [{}]", filename.display());
        let rule_body = fs::read_to_string(&filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename.display()));
        trace!("Rule body: \n{}", rule_body);
        rules.push(Rule::from_json(&rule_body).unwrap());
    }

    info!("Loaded {} rule(s) from [{}]", rules.len(), path);

    rules
}

#[cfg(test)]
extern crate serde_json;
#[cfg(test)]
extern crate tempfile;

#[cfg(test)]
mod test {

    use serde_json;
    use std::io::prelude::*;
    use std::os::unix::net::UnixStream;
    use tornado_common_api::Event;

    //#[test]
    fn should_write_to_socket() {
        let mut stream = UnixStream::connect("/tmp/something").expect("Should connect to socket");

        let event = Event::new(String::from("email"));
        write_to_socket(&mut stream, &event);

        //write_to_socket(&mut stream, b"hello world 2\n");
        //write_to_socket(&mut stream, b"hello world 3\n");
        //write_to_socket(&mut stream, b"hello world 4\n");
    }

    fn write_to_socket(stream: &mut UnixStream, event: &Event) {
        let event_bytes = serde_json::to_vec(event).unwrap();
        stream.write_all(&event_bytes).expect("should write to socket");
        stream.write_all(b"\n").expect("should write to socket");
    }
}
