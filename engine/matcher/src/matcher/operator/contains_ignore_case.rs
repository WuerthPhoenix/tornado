use crate::accessor::Accessor;
use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::model::InternalEvent;
use log::*;
use serde_json::Value;
use std::borrow::Borrow;
use tornado_common_api::ValueExt;

const OPERATOR_NAME: &str = "containsIgnoreCase";

/// A matching matcher.operator that evaluates whether the first argument contains the text passed
/// as second argument. If the second argument is not text, the operator will evaluate to false
#[derive(Debug)]
pub struct ContainsIgnoreCase {
    first: Accessor,
    second: Accessor,
}

impl ContainsIgnoreCase {
    pub fn build(first: Accessor, second: Accessor) -> Result<ContainsIgnoreCase, MatcherError> {
        Ok(ContainsIgnoreCase { first, second })
    }
}

impl Operator for ContainsIgnoreCase {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &InternalEvent) -> bool {
        match self.second.get(event) {
            Some(second_arg_value) => {
                match second_arg_value.get_text().map(|val| val.to_lowercase()) {
                    Some(second_arg_lowercased) => {
                        let option_first = self.first.get(event);
                        match option_first {
                            Some(first_arg_value) => match first_arg_value.borrow() {
                                Value::String(first_arg_string) => (first_arg_string
                                    .to_lowercase())
                                .contains(&second_arg_lowercased),
                                Value::Array(first_arg_array) => {
                                    first_arg_array.iter().any(|arr_el| {
                                        arr_el
                                            .get_text()
                                            .map(|arr_str| {
                                                arr_str.to_lowercase().eq(&second_arg_lowercased)
                                            })
                                            .unwrap_or(false)
                                    })
                                }
                                Value::Object(map) => map
                                    .iter()
                                    .any(|entry| entry.0.to_lowercase().eq(&second_arg_lowercased)),
                                Value::Null | Value::Bool(_) | Value::Number(_) => false,
                            },
                            None => false,
                        }
                    }
                    None => {
                        trace!("ContainsIgnoreCase - The second argument must be of type Value::Text, found: {:#?}. Evaluating to false", second_arg_value);
                        false
                    }
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
    use maplit::*;
    use serde_json::json;
    use tornado_common_api::{Event, Map};

    #[test]
    fn should_return_the_operator_name() {
        let operator = ContainsIgnoreCase {
            first: AccessorBuilder::new().build("", "").unwrap(),
            second: AccessorBuilder::new().build("", "").unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "one").unwrap(),
            AccessorBuilder::new().build("", "two").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert_eq!(
            "one",
            operator.first.get(&(&json!(event), &mut Value::Null).into()).unwrap().as_ref()
        );
        assert_eq!(
            "two",
            operator.second.get(&(&json!(event), &mut Value::Null).into()).unwrap().as_ref()
        );
    }

    #[test]
    fn should_evaluate_to_true_if_text_equals_with_diff_case_substring() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "oNe").unwrap(),
            AccessorBuilder::new().build("", "One").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_text_contains_substring() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "two or one").unwrap(),
            AccessorBuilder::new().build("", "one").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_text_contains_substring_but_diff_case() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "two or oNe").unwrap(),
            AccessorBuilder::new().build("", "One").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "test_TYPE").unwrap(),
        )
        .unwrap();

        let event = Event::new("tEST_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_text_does_not_contain_substring() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "wrong_test_type").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.type}").unwrap(),
        )
        .unwrap();

        let mut payload = Map::new();
        payload.insert("type".to_owned(), Value::String("tyPe".to_owned()));

        let event = Event::new_with_payload("TEst_Type", payload);

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.payload.1}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.2}").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_first_arg_is_bool() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.payload.value}").unwrap(),
            AccessorBuilder::new().build("", "t").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("value".to_owned(), Value::Bool(true));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_second_arg_is_bool() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "t").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.value}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("value".to_owned(), Value::Bool(true));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_first_arg_is_number() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.payload.value}").unwrap(),
            AccessorBuilder::new().build("", "9").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("TEst_type");
        event.payload.insert("value".to_owned(), json!(999.99));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_second_arg_is_number() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", "9").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.value}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("TEst_type");
        event.payload.insert("value".to_owned(), json!(999.99));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_array_contains_a_number_value() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value(
                    "",
                    &Value::Array(vec![Value::String("two or one".to_owned()), json!(999)]),
                )
                .unwrap(),
            AccessorBuilder::new().build_from_value("", &json!(999)).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_array_contains_a_bool_value() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value(
                    "",
                    &Value::Array(vec![Value::String("two or one".to_owned()), Value::Bool(true)]),
                )
                .unwrap(),
            AccessorBuilder::new().build_from_value("", &Value::Bool(true)).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_array_contains_a_text_value() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value(
                    "",
                    &Value::Array(vec![Value::String("two or ONE".to_owned()), json!(999)]),
                )
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::String("tWO or one".to_owned()))
                .unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_array_from_payload_contains_a_value() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value("", &Value::String("${event.payload.array}".to_owned()))
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::String("${event.payload.value}".to_owned()))
                .unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "array".to_owned(),
            Value::Array(vec![Value::String("tWo or oNE".to_owned()), json!(999)]),
        );
        event.payload.insert("value".to_owned(), Value::String("TWo or one".to_owned()));

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_array_does_not_contain_a_value() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value("", &Value::String("${event.payload.array}".to_owned()))
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::String("${event.payload.value}".to_owned()))
                .unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "array".to_owned(),
            Value::Array(vec![Value::String("two or one".to_owned()), json!(999)]),
        );
        event.payload.insert("value".to_owned(), Value::String("two or one or three".to_owned()));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_map_contains_a_key_with_different_case() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value("", &Value::String("${event.payload.map}".to_owned()))
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::String("${event.payload.value}".to_owned()))
                .unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "map".to_owned(),
            json!(hashmap!(
                "key_one".to_owned() => Value::Null,
                "key_TWO".to_owned() => Value::Null,
            )),
        );
        event.payload.insert("value".to_owned(), Value::String("KEY_two".to_owned()));

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_map_does_not_contain_a_key() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value("", &Value::String("${event.payload.map}".to_owned()))
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::String("${event.payload.value}".to_owned()))
                .unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "map".to_owned(),
            json!(hashmap!(
                "key_one".to_owned() => Value::Null,
                "key_two".to_owned() => Value::Null,
            )),
        );
        event.payload.insert("value".to_owned(), Value::String("key_three".to_owned()));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }
}
