//! The `tornado_engine_matcher` crate contains the event processing logic.

pub mod accessor;
pub mod config;
pub mod dispatcher;
pub mod error;
pub mod matcher;
pub mod model;
pub mod regex;
pub mod validator;

#[cfg(test)]
pub mod test_root {

    use lazy_static::lazy_static;
    use std::sync::Mutex;
    use tornado_common_logger::elastic_apm::ApmTracingConfig;
    use tornado_common_logger::{setup_logger, LogWorkerGuard, LoggerConfig};

    lazy_static! {
        static ref INITIALIZED: Mutex<Option<LogWorkerGuard>> = Mutex::new(None);
    }

    pub fn start_context() {
        let mut init = INITIALIZED.lock().unwrap();
        if init.is_none() {
            println!("Initialize context");
            let guard = start_logger();
            *init = Some(guard);
        }
    }

    fn start_logger() -> LogWorkerGuard {
        println!("Init logger");

        let conf = LoggerConfig {
            level: String::from("info,tornado=trace"),
            stdout_output: true,
            file_output_path: None,
            tracing_elastic_apm: ApmTracingConfig::default(),
        };
        setup_logger(conf).unwrap()
    }
}
