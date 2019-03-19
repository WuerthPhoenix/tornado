use crate::accessor::Accessor;
use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::model::InternalEvent;
use std::collections::HashMap;
use tornado_common_api::{cow_to_str, Value};

const OPERATOR_NAME: &str = "contain";

/// A matching matcher.operator that evaluates whether a string contains a given substring
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

    fn evaluate(
        &self,
        event: &InternalEvent,
        extracted_vars: Option<&HashMap<String, Value>>,
    ) -> bool {
        let option_text = self.text.get(event, extracted_vars);
        match cow_to_str(&option_text) {
            Some(text) => {
                let option_substring = self.substring.get(event, extracted_vars);
                match cow_to_str(&option_substring) {
                    Some(substring) => (&text).contains(substring),
                    None => false,
                }
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::accessor::AccessorBuilder;
    use std::collections::HashMap;
    use tornado_common_api::*;

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
        )
        .unwrap();

        let event = InternalEvent::new(Event::new("test_type"));

        assert_eq!("one", operator.text.get(&event, None).unwrap().as_ref());
        assert_eq!("two", operator.substring.get(&event, None).unwrap().as_ref());
    }

    #[test]
    fn should_evaluate_to_true_if_text_equals_substring() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_text_contains_substring() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"two or one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"test_type".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_text_does_not_contain_substring() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"wrong_test_type".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.type}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut payload = HashMap::new();
        payload.insert("type".to_owned(), Value::Text("type".to_owned()));

        let event = Event::new_with_payload("test_type", payload);

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"${event.payload.1}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.2}".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_value_of_type_bool() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"${event.payload.value}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"t".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("value".to_owned(), Value::Bool(true));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_value_of_type_number() {
        let operator = Contain::build(
            AccessorBuilder::new().build("", &"${event.payload.value}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"9".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("value".to_owned(), Value::Number(999.99));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

}
