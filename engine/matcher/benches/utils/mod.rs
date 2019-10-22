use std::fs;
use tornado_common_api::Event;
use tornado_engine_matcher::config::rule::Rule;

pub fn read_event_from_file(path: &str) -> Event {
    let event_body =
        fs::read_to_string(path).unwrap_or_else(|_| panic!("Unable to open the file [{}]", path));
    serde_json::from_str(&event_body).unwrap()
}

pub fn read_rule_from_file(path: &str) -> Rule {
    let rule_body =
        fs::read_to_string(path).unwrap_or_else(|_| panic!("Unable to open the file [{}]", path));
    serde_json::from_str(&rule_body).unwrap()
}
