use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::{Event, Payload};

pub mod config;

///A collector that receives an input in JSON format and allows the creation of Events using the JMESPath JSON query language.
#[derive(Default)]
pub struct JMESPathEventCollector {}

impl JMESPathEventCollector {
    pub fn new() -> JMESPathEventCollector {
        Default::default()
    }
}

impl<'a> Collector<&'a str> for JMESPathEventCollector {
    fn to_event(&self, input: &'a str) -> Result<Event, CollectorError> {
        serde_json::from_str::<tornado_common_api::Event>(&input)
            .map_err(|e| CollectorError::EventCreationError { message: format!("{}", e) })
    }
}

enum ValueProcessor {
    Expression { exp: jmespath::Expression<'static> },
    Text { text: String },
}

impl ValueProcessor {

    fn value<F>(&self, var: &jmespath::Variable, func: F) -> Result<(), CollectorError>
    where
        F: FnOnce(&str) -> Result<(), CollectorError>,
    {
        match self {
            ValueProcessor::Expression { exp } => {
                let search_result =
                    exp.search(var).map_err(|e| CollectorError::EventCreationError {
                        message: format!(
                            "Expression failed to execute. Exp: {}. Error: {}",
                            exp, e
                        ),
                    })?;
                let val = search_result.as_string().ok_or_else(|| {
                    CollectorError::EventCreationError {
                        message: format!("Cannot parse expression result as string. Exp: {}.", exp),
                    }
                })?;

                func(val)
            }
            ValueProcessor::Text { text } => func(&text),
        }
    }

}

#[cfg(test)]
mod test {

    use super::*;
    use std::fs;

    #[test]
    fn should_return_an_event() {
        // Arrange
        let event = Event::new(String::from("email"));
        let json = serde_json::to_string(&event).unwrap();

        let collector = JMESPathEventCollector::new();

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
        let collector = JMESPathEventCollector::new();

        // Act
        let result = collector.to_event(&json);

        // Assert
        assert!(result.is_err())
    }

}
