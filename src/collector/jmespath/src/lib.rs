use std::collections::HashMap;
use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::Event;
use tornado_common_api::Value;

pub mod config;

///A collector that receives an input in JSON format and allows the creation of Events using the JMESPath JSON query language.
pub struct JMESPathEventCollector {
    processor: EventProcessor,
}

impl JMESPathEventCollector {
    pub fn build(
        config: &config::JMESPathEventCollectorConfig,
    ) -> Result<JMESPathEventCollector, CollectorError> {
        let processor = EventProcessor::build(config)?;
        Ok(JMESPathEventCollector { processor })
    }
}

impl<'a> Collector<&'a str> for JMESPathEventCollector {
    fn to_event(&self, input: &'a str) -> Result<Event, CollectorError> {
        let data = jmespath::Variable::from_json(input).map_err(|err| {
            CollectorError::EventCreationError {
                message: format!("Cannot parse received json. Err: {} - Json: {}.", err, input),
            }
        })?;
        self.processor.process(&data)
    }
}

struct EventProcessor {
    event_type: ValueProcessor,
    payload: EventProcessorPayload,
}

type EventProcessorPayload = HashMap<String, ValueProcessor>;

const EXPRESSION_START_DELIMITER: &str = "${";
const EXPRESSION_END_DELIMITER: &str = "}";

impl EventProcessor {
    pub fn build(
        config: &config::JMESPathEventCollectorConfig,
    ) -> Result<EventProcessor, CollectorError> {
        let mut processor = EventProcessor {
            event_type: EventProcessor::build_value(&config.event_type)?,
            payload: HashMap::new(),
        };

        for (key, value) in &config.payload {
            processor.payload.insert(key.to_owned(), EventProcessor::build_value(value)?);
        }

        Ok(processor)
    }

    fn build_value(value: &str) -> Result<ValueProcessor, CollectorError> {
        if value.starts_with(EXPRESSION_START_DELIMITER)
            && value.ends_with(EXPRESSION_END_DELIMITER)
        {
            let expression = &value
                [EXPRESSION_START_DELIMITER.len()..(value.len() - EXPRESSION_END_DELIMITER.len())];
            Ok(ValueProcessor::Expression { exp: jmespath::compile(expression).unwrap() })
        } else {
            Ok(ValueProcessor::Text { text: value.to_owned() })
        }
    }

    pub fn process(&self, var: &jmespath::Variable) -> Result<Event, CollectorError> {
        let mut event = Event::new("");
        self.event_type.process(var, |value| {
            event.event_type = value.to_owned();
            Ok(())
        })?;

        for (key, value_processor) in &self.payload {
            value_processor.process(var, |value| {
                event.payload.insert(key.clone(), Value::Text(value.to_owned()));
                Ok(())
            })?;
        }

        Ok(event)
    }
}

#[derive(Debug, PartialEq)]
enum ValueProcessor {
    Expression { exp: jmespath::Expression<'static> },
    Text { text: String },
}

impl ValueProcessor {
    pub fn process<F>(&self, var: &jmespath::Variable, func: F) -> Result<(), CollectorError>
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
    use std::collections::HashMap;
    use std::fs;
    use std::rc::Rc;
    use std::sync::Mutex;

    #[test]
    fn value_processor_text_should_return_static_text() {
        // Arrange
        let value_proc = ValueProcessor::Text { text: "hello world".to_owned() };
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
        let result = value_proc.process(&data, move |value| {
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
        let value_proc = ValueProcessor::Expression { exp };
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
        let result = value_proc.process(&data, move |value| {
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
        let value_proc = ValueProcessor::Expression { exp };
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
        let result = value_proc.process(&data, move |value| {
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
        let value_proc = ValueProcessor::Expression { exp };
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
        let result = value_proc.process(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_err());
        assert_eq!("", *atomic.lock().unwrap());
    }

    /*
    #[test]
    fn value_processor_expression_should_handle_non_string_values() {
        // Arrange
        let exp = jmespath::compile("key").unwrap();
        let value_proc = ValueProcessor::Expression { exp };
        let json = r#"
        {
            "key": true
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();
        let atomic = Rc::new(Mutex::new("".to_owned()));
        let atomic_clone = atomic.clone();

        // Act
        let result = value_proc.process(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_ok());
        assert_eq!("true", *atomic.lock().unwrap());
    }
    */

    #[test]
    fn event_processor_should_build_from_config_with_static_type() {
        // Arrange
        let mut config = config::JMESPathEventCollectorConfig {
            event_type: "hello world".to_owned(),
            payload: HashMap::new(),
        };
        config.payload.insert("one".to_owned(), "value_one".to_owned());
        config.payload.insert("two".to_owned(), "value_two".to_owned());

        // Act
        let event_processor = EventProcessor::build(&config).unwrap();

        // Assert
        assert_eq!(
            ValueProcessor::Text { text: "hello world".to_owned() },
            event_processor.event_type
        );
        assert_eq!(
            &ValueProcessor::Text { text: "value_one".to_owned() },
            event_processor.payload.get("one").unwrap()
        );
        assert_eq!(
            &ValueProcessor::Text { text: "value_two".to_owned() },
            event_processor.payload.get("two").unwrap()
        );
    }

    #[test]
    fn event_processor_should_build_from_config_with_expression() {
        // Arrange
        let mut config = config::JMESPathEventCollectorConfig {
            event_type: "${first.second[0]}".to_owned(),
            payload: HashMap::new(),
        };
        config.payload.insert("one".to_owned(), "${first.third}".to_owned());
        let expected_event_expression = jmespath::compile("first.second[0]").unwrap();
        let expected_payload_expression = jmespath::compile("first.third").unwrap();

        // Act
        let event_processor = EventProcessor::build(&config).unwrap();

        // Assert
        assert_eq!(
            ValueProcessor::Expression { exp: expected_event_expression },
            event_processor.event_type
        );
        assert_eq!(
            &ValueProcessor::Expression { exp: expected_payload_expression },
            event_processor.payload.get("one").unwrap()
        );
    }

    #[test]
    fn verify_expected_io() {
        verify_io(
            "./test_resources/01_config.json",
            "./test_resources/01_input.json",
            "./test_resources/01_output.json",
        );
        verify_io(
            "./test_resources/02_config.json",
            "./test_resources/02_input.json",
            "./test_resources/02_output.json",
        );
        verify_io(
            "./test_resources/github_webhook_01_config.json",
            "./test_resources/github_webhook_01_input.json",
            "./test_resources/github_webhook_01_output.json",
        );
    }

    fn verify_io(config_path: &str, input_path: &str, output_path: &str) {
        // Arrange
        let config_json = fs::read_to_string(config_path)
            .expect(&format!("Unable to open the file [{}]", config_path));
        let config: config::JMESPathEventCollectorConfig = serde_json::from_str(&config_json)
            .map_err(|e| panic!("Cannot parse config json. Err: {}", e))
            .unwrap();

        let collector = JMESPathEventCollector::build(&config).unwrap();

        let input_json = fs::read_to_string(input_path)
            .expect(&format!("Unable to open the file [{}]", input_path));

        let output_json = fs::read_to_string(output_path)
            .expect(&format!("Unable to open the file [{}]", output_path));
        let mut expected_event: Event = serde_json::from_str(&output_json)
            .map_err(|e| panic!("Cannot parse output json. Err: {}", e))
            .unwrap();;

        // Act
        let result = collector.to_event(&input_json);

        // Assert
        assert!(result.is_ok());

        let result_event = result.unwrap();
        expected_event.created_ts = result_event.created_ts.clone();

        assert_eq!(expected_event, result_event);
    }

}
