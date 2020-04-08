use crate::accessor::Accessor;
use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::model::InternalEvent;
use log::*;
use std::borrow::Borrow;
use tornado_common_api::Value;

const OPERATOR_NAME: &str = "containIgnoreCase";

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

    fn evaluate(&self, event: &InternalEvent, extracted_vars: Option<&Value>) -> bool {
        match self.second.get(event, extracted_vars) {
            Some(second_arg_value) => match second_arg_value.as_ref() {
                Value::Text(second_arg_string) => {
                    let option_first = self.first.get(event, extracted_vars);
                    match option_first {
                        Some(first_arg_value) => match first_arg_value.borrow() {
                            Value::Text(first_arg_string) => (first_arg_string.to_lowercase())
                                .contains(&second_arg_string.to_lowercase()),
                            Value::Array(first_arg_array) => first_arg_array.iter().any(|arr_el| {
                                arr_el
                                    .get_text()
                                    .and_then(|arr_str| {
                                        Some(
                                            arr_str
                                                .to_lowercase()
                                                .eq(&second_arg_string.to_lowercase()),
                                        )
                                    })
                                    .unwrap_or(false)
                            }),
                            Value::Map(map) => map.iter().any(|entry| {
                                entry.0.to_lowercase().eq(&second_arg_string.to_lowercase())
                            }),
                            Value::Null | Value::Bool(_) | Value::Number(_) => false,
                        },
                        None => false,
                    }
                }
                Value::Null
                | Value::Bool(_)
                | Value::Number(_)
                | Value::Array(_)
                | Value::Map(_) => {
                    debug!("ContainsIgnoreCase - The second argument {:#?} must be of type Value::Text, evaluating to false", second_arg_value);
                    false
                }
            },
            None => false,
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::accessor::AccessorBuilder;
    use maplit::*;
    use std::collections::HashMap;
    use tornado_common_api::*;

    #[test]
    fn should_return_the_operator_name() {
        let operator = ContainsIgnoreCase {
            first: AccessorBuilder::new().build("", &"".to_owned()).unwrap(),
            second: AccessorBuilder::new().build("", &"".to_owned()).unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"two".to_owned()).unwrap(),
        )
        .unwrap();

        let event = InternalEvent::new(Event::new("test_type"));

        assert_eq!("one", operator.first.get(&event, None).unwrap().as_ref());
        assert_eq!("two", operator.second.get(&event, None).unwrap().as_ref());
    }

    #[test]
    fn should_evaluate_to_true_if_text_equals_with_diff_case_substring() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"oNe".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"One".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_text_contains_substring() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"two or one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_text_contains_substring_but_diff_case() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"two or oNe".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"One".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"test_TYPE".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("tEST_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_text_does_not_contain_substring() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"wrong_test_type".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.type}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut payload = HashMap::new();
        payload.insert("type".to_owned(), Value::Text("tyPe".to_owned()));

        let event = Event::new_with_payload("TEst_Type", payload);

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"${event.payload.1}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.2}".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_first_arg_is_bool() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"${event.payload.value}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"t".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("value".to_owned(), Value::Bool(true));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_second_arg_is_bool() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"t".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.value}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("value".to_owned(), Value::Bool(true));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_first_arg_is_number() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"${event.payload.value}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"9".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("TEst_type");
        event.payload.insert("value".to_owned(), Value::Number(Number::Float(999.99)));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_second_arg_is_number() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new().build("", &"9".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.value}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("TEst_type");
        event.payload.insert("value".to_owned(), Value::Number(Number::Float(999.99)));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_array_contains_a_number_value() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value(
                    "",
                    &Value::Array(vec![
                        Value::Text("two or one".to_owned()),
                        Value::Number(Number::PosInt(999)),
                    ]),
                )
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::Number(Number::PosInt(999)))
                .unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_array_contains_a_bool_value() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value(
                    "",
                    &Value::Array(vec![Value::Text("two or one".to_owned()), Value::Bool(true)]),
                )
                .unwrap(),
            AccessorBuilder::new().build_from_value("", &Value::Bool(true)).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_array_contains_a_text_value() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value(
                    "",
                    &Value::Array(vec![
                        Value::Text("two or ONE".to_owned()),
                        Value::Number(Number::PosInt(999)),
                    ]),
                )
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::Text("tWO or one".to_owned()))
                .unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_array_from_payload_contains_a_value() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value("", &Value::Text("${event.payload.array}".to_owned()))
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::Text("${event.payload.value}".to_owned()))
                .unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "array".to_owned(),
            Value::Array(vec![
                Value::Text("tWo or oNE".to_owned()),
                Value::Number(Number::PosInt(999)),
            ]),
        );
        event.payload.insert("value".to_owned(), Value::Text("TWo or one".to_owned()));

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_array_does_not_contain_a_value() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value("", &Value::Text("${event.payload.array}".to_owned()))
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::Text("${event.payload.value}".to_owned()))
                .unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "array".to_owned(),
            Value::Array(vec![
                Value::Text("two or one".to_owned()),
                Value::Number(Number::PosInt(999)),
            ]),
        );
        event.payload.insert("value".to_owned(), Value::Text("two or one or three".to_owned()));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_map_contains_a_key_with_different_case() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value("", &Value::Text("${event.payload.map}".to_owned()))
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::Text("${event.payload.value}".to_owned()))
                .unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "map".to_owned(),
            Value::Map(hashmap!(
                "key_one".to_owned() => Value::Null,
                "key_TWO".to_owned() => Value::Null,
            )),
        );
        event.payload.insert("value".to_owned(), Value::Text("KEY_two".to_owned()));

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_map_does_not_contain_a_key() {
        let operator = ContainsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value("", &Value::Text("${event.payload.map}".to_owned()))
                .unwrap(),
            AccessorBuilder::new()
                .build_from_value("", &Value::Text("${event.payload.value}".to_owned()))
                .unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "map".to_owned(),
            Value::Map(hashmap!(
                "key_one".to_owned() => Value::Null,
                "key_two".to_owned() => Value::Null,
            )),
        );
        event.payload.insert("value".to_owned(), Value::Text("key_three".to_owned()));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }
}
