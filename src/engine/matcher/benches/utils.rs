use std::fs;
use tornado_common_api::Event;
use tornado_engine_matcher::config::Rule;


pub fn read_event_from_file(path: &str) -> Event {
    info!("Loading event from file: [{}]", path);
    let event_body = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Unable to open the file [{}]", path));
    trace!("Event body: \n{}", event_body);
    serde_json::from_str(&event_body).unwrap()
}

pub fn read_rule_from_file(path: &str) -> Rule {
    info!("Loading rule from file: [{}]", path);
    let rule_body = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Unable to open the file [{}]", path));
    trace!("Rule body: \n{}", rule_body);
    serde_json::from_str(&rule_body).unwrap()
}


pub mod logger {

    use std::collections::HashMap;
    use std::sync::Mutex;
    use tornado_common_logger::{setup_logger, LoggerConfig};

    lazy_static! {
        static ref INITIALIZED: Mutex<bool> = Mutex::new(false);
    }

    pub fn start() {
        let mut init = INITIALIZED.lock().unwrap();
        if !*init {
            println!("Initialize logger");
            start_logger_configuration();
            *init = true;
        }
    }

    fn start_logger_configuration() {
        println!("Init logger");

        let conf = LoggerConfig {
            root_level: String::from("trace"),
            output_system_enabled: true,
            output_file_enabled: false,
            output_file_name: String::from(""),
            module_level: HashMap::new(),
        };
        setup_logger(&conf).unwrap();
    }

}