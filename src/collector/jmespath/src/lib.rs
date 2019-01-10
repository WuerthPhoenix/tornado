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


struct EventProcessor {
    event_type: ValueProcessor
}

const EXPRESSION_START_DELIMITER: &str = "${";
const EXPRESSION_END_DELIMITER: &str = "}";

impl EventProcessor {

    pub fn build(config: &config::JMESPathEventCollectorConfig) -> Result<EventProcessor, CollectorError> {
        Ok(EventProcessor{event_type: EventProcessor::build_value(&config.event_type)?})
    }

    fn build_value(value: &str)  -> Result<ValueProcessor, CollectorError> {
        if value.starts_with( EXPRESSION_START_DELIMITER) && value.ends_with( EXPRESSION_END_DELIMITER) {
            let expression= & value[ EXPRESSION_START_DELIMITER.len()..(value.len() - EXPRESSION_END_DELIMITER.len())];
            Ok(ValueProcessor::Expression{exp: jmespath::compile(expression).unwrap()})
        } else {
            Ok(ValueProcessor::Text {text: value.to_owned()})
        }
    }
}

#[derive(Debug, PartialEq)]
enum ValueProcessor {
    Expression { exp: jmespath::Expression<'static> },
    Text { text: String },
}

impl ValueProcessor {

    pub fn get_value<F>(&self, var: &jmespath::Variable, func: F) -> Result<(), CollectorError>
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
    use std::sync::Mutex;
    use std::rc::Rc;

    #[test]
    fn value_processor_text_should_return_static_text() {
        // Arrange
        let value_proc = ValueProcessor::Text {text: "hello world".to_owned()};
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();
        let atomic = Rc::new(Mutex::new("".to_owned()));
        let atomic_clone = atomic.clone();

        // Act
        let result = value_proc.get_value(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_ok());
        assert_eq!("hello world", *atomic.lock().unwrap());

    }

    #[test]
    fn value_processor_expression_should_return_from_json() {
        // Arrange
        let exp = jmespath::compile("level_one.level_two").unwrap();
        let value_proc = ValueProcessor::Expression {exp};
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();
        let atomic = Rc::new(Mutex::new("".to_owned()));
        let atomic_clone = atomic.clone();

        // Act
        let result = value_proc.get_value(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_ok());
        assert_eq!("level_two_value", *atomic.lock().unwrap());

    }

    #[test]
    fn value_processor_expression_should_return_error_if_not_present() {
        // Arrange
        let exp = jmespath::compile("level_one.level_three").unwrap();
        let value_proc = ValueProcessor::Expression {exp};
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();
        let atomic = Rc::new(Mutex::new("".to_owned()));
        let atomic_clone = atomic.clone();

        // Act
        let result = value_proc.get_value(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_err());
        assert_eq!("", *atomic.lock().unwrap());

    }

    #[test]
    fn value_processor_expression_should_return_error_if_not_present_in_array() {
        // Arrange
        let exp = jmespath::compile("level_one.level_two[2]").unwrap();
        let value_proc = ValueProcessor::Expression {exp};
        let json = r#"
        {
            "level_one": {
                "level_two": ["level_two_0", "level_two_1"]
            }
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();
        let atomic = Rc::new(Mutex::new("".to_owned()));
        let atomic_clone = atomic.clone();

        // Act
        let result = value_proc.get_value(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_err());
        assert_eq!("", *atomic.lock().unwrap());

    }

    #[test]
    fn event_processor_should_build_from_config_with_static_type() {
        // Arrange
        let config = config::JMESPathEventCollectorConfig{event_type: "hello world".to_owned()};

        // Act
        let event_processor = EventProcessor::build(&config).unwrap();

        // Assert
        assert_eq!(ValueProcessor::Text {text: "hello world".to_owned()}, event_processor.event_type);

    }

    #[test]
    fn event_processor_should_build_from_config_with_expression() {
        // Arrange
        let config = config::JMESPathEventCollectorConfig{event_type: "${first.second[0]}".to_owned()};
        let expected_expression = jmespath::compile("first.second[0]").unwrap();

        // Act
        let event_processor = EventProcessor::build(&config).unwrap();

        // Assert
        assert_eq!(ValueProcessor::Expression {exp: expected_expression}, event_processor.event_type);

    }

}
