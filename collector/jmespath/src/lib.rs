use jmespath::Rcvar;
use log::trace;
use serde_json::json;
use serde_json::Map;
use std::collections::HashMap;
use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::Event;
use tornado_common_api::Payload;
use tornado_common_api::Value;
use tornado_common_api::ValueExt;

pub mod config;

/// A Collector that receives an input in JSON format and allows the creation of Events
///   using the JMESPath JSON query language.
pub struct JMESPathEventCollector {
    processor: EventProcessor,
}

impl JMESPathEventCollector {
    /// Builds a new Collector instance.
    pub fn build(
        config: config::JMESPathEventCollectorConfig,
    ) -> Result<JMESPathEventCollector, CollectorError> {
        let processor = EventProcessor::build(config)?;
        Ok(JMESPathEventCollector { processor })
    }
}

impl<'a> Collector<&'a str> for JMESPathEventCollector {
    fn to_event(&self, input: &'a str) -> Result<Event, CollectorError> {
        trace!("JMESPathEventCollector - received event: {}", input);

        let data = jmespath::Variable::from_json(input).map_err(|err| {
            CollectorError::EventCreationError {
                message: format!("Cannot parse received json. Err: {:?} - Json: {}.", err, input),
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
        config: config::JMESPathEventCollectorConfig,
    ) -> Result<EventProcessor, CollectorError> {
        let mut processor = EventProcessor {
            event_type: EventProcessor::build_value_processor(Value::String(config.event_type))?,
            payload: EventProcessorPayload::new(),
        };

        for (key, value) in config.payload {
            processor.payload.insert(key, EventProcessor::build_value_processor(value)?);
        }

        Ok(processor)
    }

    fn build_value_processor(value: Value) -> Result<ValueProcessor, CollectorError> {
        match value {
            Value::Object(payload) => {
                let mut processor_payload = HashMap::new();
                for (key, value) in payload {
                    processor_payload.insert(key, EventProcessor::build_value_processor(value)?);
                }
                Ok(ValueProcessor::Map(processor_payload))
            }
            Value::Array(values) => {
                let mut processor_values = vec![];
                for value in values {
                    processor_values.push(EventProcessor::build_value_processor(value)?)
                }
                Ok(ValueProcessor::Array(processor_values))
            }
            Value::String(text) => EventProcessor::build_value_processor_from_str(&text),
            Value::Bool(boolean) => Ok(ValueProcessor::Bool(boolean)),
            Value::Number(number) => {
                Ok(ValueProcessor::Number(number.as_f64().unwrap_or_default()))
            }
            Value::Null => Ok(ValueProcessor::Null),
        }
    }

    fn build_value_processor_from_str(text: &str) -> Result<ValueProcessor, CollectorError> {
        if text.starts_with(EXPRESSION_START_DELIMITER) && text.ends_with(EXPRESSION_END_DELIMITER)
        {
            let expression = &text
                [EXPRESSION_START_DELIMITER.len()..(text.len() - EXPRESSION_END_DELIMITER.len())];
            let jmespath_exp = jmespath::compile(expression).map_err(|err| {
                CollectorError::CollectorCreationError {
                    message: format!(
                        "Not valid jmespath expression: [{}]. Err: {:?}",
                        expression, err
                    ),
                }
            })?;
            Ok(ValueProcessor::Expression { exp: jmespath_exp })
        } else {
            Ok(ValueProcessor::Text(text.to_owned()))
        }
    }

    pub fn process(&self, var: &jmespath::Variable) -> Result<Event, CollectorError> {
        let event_type = self
            .event_type
            .process(var)?
            .get_text()
            .ok_or(CollectorError::EventCreationError {
                message: "Event type must be a string".to_owned(),
            })?
            .to_owned();
        let mut event = Event::new(event_type);

        for (key, value_processor) in &self.payload {
            event.payload.insert(key.clone(), value_processor.process(var)?);
        }

        Ok(event)
    }
}

#[derive(Debug, PartialEq)]
enum ValueProcessor {
    Expression { exp: jmespath::Expression<'static> },
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
    Array(Vec<ValueProcessor>),
    Map(HashMap<String, ValueProcessor>),
}

impl ValueProcessor {
    pub fn process(&self, var: &jmespath::Variable) -> Result<Value, CollectorError> {
        match self {
            ValueProcessor::Expression { exp } => {
                let search_result =
                    exp.search(var).map_err(|e| CollectorError::EventCreationError {
                        message: format!(
                            "Expression failed to execute. Exp: {}. Error: {}",
                            exp, e
                        ),
                    })?;
                variable_to_value(&search_result)
            }
            ValueProcessor::Null => Ok(Value::Null),
            ValueProcessor::Text(text) => Ok(Value::String(text.to_owned())),
            ValueProcessor::Number(number) => Ok(json!(*number)),
            ValueProcessor::Bool(boolean) => Ok(Value::Bool(*boolean)),
            ValueProcessor::Map(payload) => {
                let mut processor_payload = Map::new();
                for (key, value) in payload {
                    processor_payload.insert(key.to_owned(), value.process(var)?);
                }
                Ok(Value::Object(processor_payload))
            }
            ValueProcessor::Array(values) => {
                let mut processor_values = vec![];
                for value in values {
                    processor_values.push(value.process(var)?)
                }
                Ok(Value::Array(processor_values))
            }
        }
    }
}

fn variable_to_value(var: &Rcvar) -> Result<Value, CollectorError> {
    match var.as_ref() {
        jmespath::Variable::String(s) => Ok(Value::String(s.to_owned())),
        jmespath::Variable::Bool(b) => Ok(Value::Bool(*b)),
        jmespath::Variable::Number(n) => Ok(json!(n)),
        jmespath::Variable::Object(values) => {
            let mut payload = Payload::new();
            for (key, value) in values {
                payload.insert(key.to_owned(), variable_to_value(value)?);
            }
            Ok(Value::Object(payload))
        }
        jmespath::Variable::Array(ref values) => {
            let mut payload = vec![];
            for value in values {
                payload.push(variable_to_value(value)?);
            }
            Ok(Value::Array(payload))
        }
        jmespath::Variable::Null => Ok(Value::Null),
        // ToDo: how to map Expref?
        jmespath::Variable::Expref(_) => Err(CollectorError::EventCreationError {
            message: "Cannot map jmespath::Variable::Expref to the Event payload".to_owned(),
        }),
    }
}

#[cfg(test)]
mod test {

    use serde_json::{json, Map};

    use super::*;
    use std::collections::HashMap;
    use std::fs;

    #[test]
    fn value_processor_text_should_return_static_text() {
        // Arrange
        let value_proc = ValueProcessor::Text("hello world".to_owned());
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();

        // Act
        let result = value_proc.process(&data);

        // Assert
        assert!(result.is_ok());
        assert_eq!(Value::String("hello world".to_owned()), result.unwrap());
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

        // Act
        let result = value_proc.process(&data);

        // Assert
        assert!(result.is_ok());
        assert_eq!(Value::String("level_two_value".to_owned()), result.unwrap());
    }

    #[test]
    fn value_processor_expression_should_return_null_if_not_present() {
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

        // Act
        let result = value_proc.process(&data);

        // Assert
        assert!(result.is_ok());
        assert_eq!(Value::Null, result.unwrap());
    }

    #[test]
    fn value_processor_expression_should_return_null_if_not_present_in_array() {
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

        // Act
        let result = value_proc.process(&data);

        // Assert
        assert!(result.is_ok());
        assert_eq!(Value::Null, result.unwrap());
    }

    #[test]
    fn value_processor_expression_should_handle_boolean_values() {
        // Arrange
        let exp = jmespath::compile("key").unwrap();
        let value_proc = ValueProcessor::Expression { exp };
        let json = r#"
        {
            "key": true
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();

        // Act
        let result = value_proc.process(&data);

        // Assert
        assert!(result.is_ok());
        assert_eq!(Value::Bool(true), result.unwrap());
    }

    #[test]
    fn value_processor_expression_should_handle_numeric_values() {
        // Arrange
        let exp = jmespath::compile("key").unwrap();
        let value_proc = ValueProcessor::Expression { exp };
        let json = r#"
        {
            "key": 99.66
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();

        // Act
        let result = value_proc.process(&data);

        // Assert
        assert!(result.is_ok());
        assert_eq!(json!(99.66), result.unwrap());
    }

    #[test]
    fn value_processor_expression_should_handle_arrays() {
        // Arrange
        let exp = jmespath::compile("key").unwrap();
        let value_proc = ValueProcessor::Expression { exp };
        let json = r#"
        {
            "key": ["one", true, 13]
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();

        // Act
        let result = value_proc.process(&data);

        // Assert
        assert!(result.is_ok());
        assert_eq!(
            Value::Array(vec![Value::String("one".to_owned()), Value::Bool(true), json!(13)]),
            result.unwrap()
        );
    }

    #[test]
    fn value_processor_expression_should_handle_maps() {
        // Arrange
        let exp = jmespath::compile("key").unwrap();
        let value_proc = ValueProcessor::Expression { exp };
        let json = r#"
        {
            "key": {
                "one": true,
                "two": 13.0
            }
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();

        // Act
        let result = value_proc.process(&data);

        // Assert
        assert!(result.is_ok());

        let mut payload = Map::new();
        payload.insert("one".to_owned(), Value::Bool(true));
        payload.insert("two".to_owned(), json!(13.0));

        assert_eq!(Value::Object(payload), result.unwrap());
    }

    #[test]
    fn event_processor_should_build_from_config_with_static_type() {
        // Arrange
        let mut config = config::JMESPathEventCollectorConfig {
            event_type: "hello world".to_owned(),
            payload: HashMap::new(),
        };
        config.payload.insert("one".to_owned(), Value::String("value_one".to_owned()));
        config.payload.insert("two".to_owned(), Value::String("value_two".to_owned()));

        // Act
        let event_processor = EventProcessor::build(config).unwrap();

        // Assert
        assert_eq!(ValueProcessor::Text("hello world".to_owned()), event_processor.event_type);
        assert_eq!(
            &ValueProcessor::Text("value_one".to_owned()),
            event_processor.payload.get("one").unwrap()
        );
        assert_eq!(
            &ValueProcessor::Text("value_two".to_owned()),
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
        config.payload.insert("one".to_owned(), Value::String("${first.third}".to_owned()));
        let expected_event_expression = jmespath::compile("first.second[0]").unwrap();
        let expected_payload_expression = jmespath::compile("first.third").unwrap();

        // Act
        let event_processor = EventProcessor::build(config).unwrap();

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
    fn event_processor_should_build_from_config_with_recursive_maps() {
        // Arrange
        let mut config = config::JMESPathEventCollectorConfig {
            event_type: "type".to_owned(),
            payload: HashMap::new(),
        };
        config.payload.insert("one".to_owned(), Value::String("${first.third}".to_owned()));

        let mut inner_map = Map::new();
        inner_map.insert("two".to_owned(), Value::String("${first.second[0]}".to_owned()));
        config.payload.insert("two".to_owned(), Value::Object(inner_map));

        let expected_payload_expression_one = jmespath::compile("first.third").unwrap();
        let expected_payload_expression_two = jmespath::compile("first.second[0]").unwrap();

        // Act
        let event_processor = EventProcessor::build(config).unwrap();

        // Assert
        assert_eq!(
            &ValueProcessor::Expression { exp: expected_payload_expression_one },
            event_processor.payload.get("one").unwrap()
        );

        let mut inner_processor = HashMap::new();
        inner_processor.insert(
            "two".to_owned(),
            ValueProcessor::Expression { exp: expected_payload_expression_two },
        );
        assert_eq!(
            &ValueProcessor::Map(inner_processor),
            event_processor.payload.get("two").unwrap()
        );
    }

    #[test]
    fn event_processor_should_build_from_config_with_recursive_arrays() {
        // Arrange
        let mut config = config::JMESPathEventCollectorConfig {
            event_type: "type".to_owned(),
            payload: HashMap::new(),
        };

        let mut inner_array = vec![];
        inner_array.push(Value::String("${first.second[0]}".to_owned()));
        config.payload.insert("array".to_owned(), Value::Array(inner_array));

        let expected_payload_expression = jmespath::compile("first.second[0]").unwrap();

        // Act
        let event_processor = EventProcessor::build(config).unwrap();

        let mut inner_processor = vec![];
        inner_processor.push(ValueProcessor::Expression { exp: expected_payload_expression });
        assert_eq!(
            &ValueProcessor::Array(inner_processor),
            event_processor.payload.get("array").unwrap()
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
            .map_err(|e| panic!("Cannot parse config json. Err: {:?}", e))
            .unwrap();

        let collector = JMESPathEventCollector::build(config).unwrap();

        let input_json = fs::read_to_string(input_path)
            .expect(&format!("Unable to open the file [{}]", input_path));

        let output_json = fs::read_to_string(output_path)
            .expect(&format!("Unable to open the file [{}]", output_path));
        let mut expected_event: Event = serde_json::from_str(&output_json)
            .map_err(|e| panic!("Cannot parse output json. Err: {:?}", e))
            .unwrap();

        // Act
        let result = collector.to_event(&input_json);

        // Assert
        assert!(result.is_ok());

        let result_event = result.unwrap();
        expected_event.created_ms = result_event.created_ms.clone();

        assert_eq!(expected_event, result_event);
    }
}
