use futures::sync::mpsc;
use log::*;
use std::sync::Arc;
use std::thread;
use tokio::prelude::*;
use tokio::runtime::Runtime;
use tornado_collector_common::Collector;
use tornado_collector_json::JsonEventCollector;
use tornado_common_logger::setup_logger;
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::dispatcher::Dispatcher;
use tornado_engine_matcher::matcher::Matcher;
use tornado_network_simple::SimpleEventBus;

mod config;
mod io;

fn main() {
    let conf = config::Conf::build();
    setup_logger(&conf.logger).unwrap();

    // Load rules from fs
    let config_rules = MatcherConfig::read_from_dir(&conf.io.rules_dir).unwrap();

    // Start matcher & dispatcher
    let matcher = Arc::new(Matcher::build(&config_rules).unwrap());
    let collector = Arc::new(JsonEventCollector::new());

    // Configure action dispatcher
    let event_bus = {
        let event_bus = SimpleEventBus::new();

        /*
        let executor = LoggerExecutor::new();
        event_bus.subscribe_to_action(
            "Logger",
            Box::new(move |action| match executor.execute(&action) {
                Ok(_) => {}
                Err(e) => error!("Cannot log action: {}", e),
            }),
        );
        */
        Arc::new(event_bus)
    };

    let mut runtime = Runtime::new().unwrap();

    let (tx, rx) = mpsc::unbounded::<String>();

    runtime.spawn(rx.for_each(move |line| {
        debug!("Client - Thread {:?} - Received line {}", thread::current().name(), line);

        match collector.to_event(&line) {
            Ok(event) => {
                let matcher_clone = matcher.clone();
                let event_bus_clone = event_bus.clone();
                tokio::spawn(future::lazy(move || {
                    debug!(
                        "Client - Thread {:?} - Got event!! span matcher thread",
                        thread::current().name()
                    );
                    let processed_event = matcher_clone.process(event);
                    match Dispatcher::build(event_bus_clone)
                        .unwrap()
                        .dispatch_actions(processed_event.result)
                    {
                        Ok(_) => {}
                        Err(e) => error!("Cannot dispatch action: {}", e),
                    };
                    Ok(())
                }));
            }
            Err(e) => error!(
                "JsonReaderActor - {:?} - Cannot unmarshal event from json: {}",
                thread::current().name(),
                e
            ),
        };

        Ok(())
    }));

    let server = io::uds::start_uds_socket(conf.io.uds_path, tx);
    runtime
        .block_on(server.map_err(|e| panic!("err={:?}", e)))
        .expect("Tokio runtime should start");
}
