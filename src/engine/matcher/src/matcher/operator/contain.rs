use accessor::Accessor;
use error::MatcherError;
use matcher::operator::Operator;
use model::ProcessedEvent;

const OPERATOR_NAME: &str = "contain";

/// A matching matcher.operator that evaluates whether a string contains a substring
#[derive(Debug)]
pub struct Contain {
    text: Accessor,
    substring: Accessor,
}

impl Contain {
    pub fn build(text: Accessor, substring: Accessor) -> Result<Contain, MatcherError> {
        Ok(Contain { text, substring })
    }
}

impl Operator for Contain {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &ProcessedEvent) -> bool {
        match self.text.get(event) {
            Some(text) => match self.substring.get(event) {
                Some(substring) => (&text).contains(substring.as_ref()),
                None => false,
            },
            None => false,
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use accessor::AccessorBuilder;
    use std::collections::HashMap;
    use tornado_common_api::Event;

    #[test]
    fn should_return_the_operator_name() {
        let operator = Contain {
            text: AccessorBuilder::new().build("", &"".to_owned()).unwrap(),
            substring: AccessorBuilder::new().build("", &"".to_owned()).unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"two".to_owned()).unwrap(),
        ).unwrap();

        let event = ProcessedEvent::new(Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        });

        assert_eq!("one", operator.text.get(&event).unwrap());
        assert_eq!("two", operator.substring.get(&event).unwrap());
    }

    #[test]
    fn should_evaluate_to_true_if_text_equals_substring() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
        ).unwrap();

        let event = Event { payload: HashMap::new(), event_type: "".to_owned(), created_ts: 0 };

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_to_true_if_text_contains_substring() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"two or one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
        ).unwrap();

        let event = Event { payload: HashMap::new(), event_type: "".to_owned(), created_ts: 0 };

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"test_type".to_owned()).unwrap(),
        ).unwrap();

        let event =
            Event { payload: HashMap::new(), event_type: "test_type".to_owned(), created_ts: 0 };

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_to_false_if_text_does_not_contain_substring() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"wrong_test_type".to_owned()).unwrap(),
        ).unwrap();

        let event =
            Event { payload: HashMap::new(), event_type: "test_type".to_owned(), created_ts: 0 };

        assert!(!operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.type}".to_owned()).unwrap(),
        ).unwrap();

        let mut payload = HashMap::new();
        payload.insert("type".to_owned(), "type".to_owned());

        let event = Event { payload, event_type: "event type".to_owned(), created_ts: 0 };

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"${event.payload.1}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.2}".to_owned()).unwrap(),
        ).unwrap();

        let event = Event { payload: HashMap::new(), event_type: "type".to_owned(), created_ts: 0 };

        assert!(!operator.evaluate(&ProcessedEvent::new(event)));
    }

}