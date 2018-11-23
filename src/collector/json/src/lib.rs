extern crate serde_json;
extern crate tornado_collector_common;
extern crate tornado_common_api;

use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::Event;

/// A collector that receives an input JSON and unmarshal it into the Event struct.
#[derive(Default)]
pub struct JsonCollector {}

impl JsonCollector {
    pub fn new() -> JsonCollector {
        Default::default()
    }
}

impl<'a> Collector<&'a str> for JsonCollector {
    fn to_event(&self, input: &'a str) -> Result<Event, CollectorError> {
        serde_json::from_str::<tornado_common_api::Event>(&input)
            .map_err(|e| CollectorError::EventCreationError { message: format!("{}", e) })
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_return_an_event() {
        // Arrange
        let event = Event::new(String::from("email"));
        let json = serde_json::to_string(&event).unwrap();

        let collector = JsonCollector::new();

        // Act
        let from_json = collector.to_event(&json).unwrap();

        // Assert
        assert_eq!(event.event_type, from_json.event_type);
        assert_eq!(event.created_ts, from_json.created_ts);
    }

    #[test]
    fn should_return_an_error() {
        // Arrange
        let json = "{message: 'hello_world'}".to_owned();
        let collector = JsonCollector::new();

        // Act
        let result = collector.to_event(&json);

        // Assert
        assert!(result.is_err())
    }

}
