extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_engine_matcher;
extern crate tornado_network_common;
extern crate tornado_network_simple;

extern crate config as config_rs;
#[macro_use]
extern crate log;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_uds;

use std::sync::Arc;
use std::fs;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tornado_common_logger::setup_logger;
use tornado_engine_matcher::config::Rule;
use tornado_engine_matcher::matcher::Matcher;

mod config;
mod reader;
mod uds;

fn main() {

    let conf = config::Conf::new().expect("Should read the configuration");

    setup_logger(&conf.logger).unwrap();

    // Load rules from fs
    let config_rules = read_rules_from_config(&conf.io.json_rules_path);

    // Start matcher & dispatcher
    let matcher = Arc::new(Matcher::new(&config_rules).unwrap());


    let server = reader::uds::start_uds_socket(conf.io.uds_socket_path);

    tokio::run(server.map_err(|e| panic!("err={:?}", e)) );

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