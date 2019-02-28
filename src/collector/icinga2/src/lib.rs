use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::{Event, Payload};

#[derive(Default)]
pub struct Icinga2EventCollector {}

impl Icinga2EventCollector {
    pub fn new() -> Icinga2EventCollector {
        Default::default()
    }
}

impl<'a> Collector<&'a str> for Icinga2EventCollector {
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
        /*
        // Arrange
        let event = Event::new(String::from("email"));
        let json = serde_json::to_string(&event).unwrap();

        let collector = Icinga2EventCollector::new();

        // Act
        let from_json = collector.to_event(&json).unwrap();

        // Assert
        assert_eq!(event.event_type, from_json.event_type);
        assert_eq!(event.created_ts, from_json.created_ts);
        */
    }

}
