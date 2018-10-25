//! The `tornado_engine_matcher` crate contains the event processing logic.
//!
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate log;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tornado_common_api;

pub mod accessor;
pub mod action;
pub mod config;
pub mod error;
pub mod extractor;
pub mod matcher;
pub mod model;
pub mod operator;
pub mod validator;

#[cfg(test)]
extern crate chrono;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
extern crate tornado_common_logger;

#[cfg(test)]
pub mod test_root {

    use std::sync::Mutex;
    use tornado_common_logger::{setup_logger, LoggerConfig};

    lazy_static! {
        static ref INITIALIZED: Mutex<bool> = Mutex::new(false);
    }

    pub fn start_context() {
        let mut init = INITIALIZED.lock().unwrap();
        if !*init {
            println!("Initialize context");
            start_logger();
            *init = true;
        }
    }

    fn start_logger() {
        println!("Init logger");

        let conf = LoggerConfig {
            root_level: String::from("trace"),
            output_system_enabled: true,
            output_file_enabled: false,
            output_file_name: String::from(""),
        };
        setup_logger(&conf).unwrap();
    }

}
