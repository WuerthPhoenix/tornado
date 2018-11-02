extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_engine_matcher;
extern crate tornado_network_common;
extern crate tornado_network_simple;

extern crate actix;
extern crate bytes;
extern crate futures;
#[macro_use] extern crate log;
extern crate serde;
extern crate serde_json;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_uds;

pub mod matcher;
pub mod uds;

#[cfg(test)]
extern crate tempfile;

use actix::prelude::*;
use futures::Stream;
use matcher::MatcherActor;
use uds::{UdsConnectMessage, UdsServerActor};
use std::fs;
use std::sync::Arc;
use tokio_uds::*;
use tornado_engine_matcher::config::Rule;
use tornado_engine_matcher::matcher::Matcher;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tornado_common_logger::{setup_logger, LoggerConfig};
use tornado_network_simple::SimpleEventBus;

fn main() {

    // Setup logger
    let conf = LoggerConfig {
        root_level: String::from("trace"),
        output_system_enabled: true,
        output_file_enabled: false,
        output_file_name: String::from(""),
    };
    setup_logger(&conf).unwrap();

    // Load rules from fs
    let config_path = "./config";
    let config_rules_path = format!("{}{}", config_path, "/rules");
    let config_rules = read_rules_from_config(&config_rules_path);

    // Start matcher & dispatcher
    let matcher = Arc::new(Matcher::new(&config_rules).unwrap());
    let event_bus = Arc::new(SimpleEventBus::new());
    let dispatcher = Arc::new(Dispatcher::new(event_bus.clone()).unwrap());

    // start system
    System::run(|| {

        // start new actor
        let matcher_actor = MatcherActor{
            dispatcher: dispatcher,
            matcher: matcher
        }.start();

        let sock_path = "/tmp/something";
        let listener = UnixListener::bind(&sock_path).unwrap();


        UdsServerActor::create(|ctx| {
            ctx.add_message_stream(listener.incoming()
                .map_err(|e| panic!("err={:?}", e))
                .map(|stream| {
                    //let addr = stream.peer_addr().unwrap();
                    UdsConnectMessage(stream)
                }));
            UdsServerActor{ matcher_addr: matcher_actor }
        });

    });

    fn read_rules_from_config(path: &str) -> Vec<Rule> {
        let paths = fs::read_dir(path).unwrap();
        let mut rules = vec![];

        for path in paths {
            let filename = path.unwrap().path();
            info!("Loading rule from file: [{}]", filename.display());
            let rule_body = fs::read_to_string(&filename).expect(&format!("Unable to open the file [{}]", filename.display()));
            trace!("Rule body: \n{}", rule_body);
            rules.push(Rule::from_json(&rule_body).unwrap());
        };

        info!("Loaded {} rule(s) from [{}]", rules.len(), path);

        rules
    }

}