use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::{accessor::Accessor, model::InternalEvent};
use tornado_common_api::{cow_to_str, Value};

const OPERATOR_NAME: &str = "contains";

/// A matching matcher.operator that evaluates whether the first argument contains the second
#[derive(Debug)]
pub struct Contains {
    first: Accessor,
    second: Accessor,
}

impl Contains {
    pub fn build(first: Accessor, second: Accessor) -> Result<Contains, MatcherError> {
        Ok(Contains { first, second })
    }
}

impl Operator for Contains {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &InternalEvent) -> bool {
        match self.first.get(event) {
            Some(first_value) => match first_value.as_ref() {
                Value::String(first) => {
                    let option_substring = self.second.get(event);
                    match cow_to_str(&option_substring) {
                        Some(substring) => first.contains(substring),
                        None => false,
                    }
                }
                Value::Array(array) => {
                    if let Some(value) = self.second.get(event) {
                        array.contains(value.as_ref())
                    } else {
                        false
                    }
                }
                Value::Object(map) => {
                    let second = self.second.get(event);
                    match cow_to_str(&second) {
                        Some(key) => map.contains_key(key),
                        None => false,
                    }
                }
                Value::Number(..) | Value::Bool(..) | Value::Null => false,
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
    use serde_json::json;
    use tornado_common_api::*;

    #[test]
    fn should_return_the_operator_name() {
        let operator = Contains {
            first: AccessorBuilder::new().build("", "").unwrap(),
            second: AccessorBuilder::new().build("", "").unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator = Contains::build(
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
    fn should_evaluate_to_true_if_text_equals_substring() {
        let operator = Contains::build(
            AccessorBuilder::new().build("", "one").unwrap(),
            AccessorBuilder::new().build("", "one").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_text_contains_substring() {
        let operator = Contains::build(
            AccessorBuilder::new().build("", "two or one").unwrap(),
            AccessorBuilder::new().build("", "one").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = Contains::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "test_type").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_text_does_not_contain_substring() {
        let operator = Contains::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "wrong_test_type").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = Contains::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.type}").unwrap(),
        )
        .unwrap();

        let mut payload = Map::new();
        payload.insert("type".to_owned(), Value::String("type".to_owned()));

        let event = Event::new_with_payload("test_type", payload);

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = Contains::build(
            AccessorBuilder::new().build("", "${event.payload.1}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.2}").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_value_of_type_bool() {
        let operator = Contains::build(
            AccessorBuilder::new().build("", "${event.payload.value}").unwrap(),
            AccessorBuilder::new().build("", "t").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("value".to_owned(), Value::Bool(true));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_value_of_type_number() {
        let operator = Contains::build(
            AccessorBuilder::new().build("", "${event.payload.value}").unwrap(),
            AccessorBuilder::new().build("", "9").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("value".to_owned(), json!(999.99));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_array_contains_a_value() {
        let operator = Contains::build(
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

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_array_from_payload_contains_a_value() {
        let operator = Contains::build(
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
        event.payload.insert("value".to_owned(), Value::String("two or one".to_owned()));

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_array_does_not_contain_a_value() {
        let operator = Contains::build(
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
    fn should_evaluate_to_true_if_map_contains_a_key() {
        let operator = Contains::build(
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
        event.payload.insert("value".to_owned(), Value::String("key_two".to_owned()));

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_map_does_not_contain_a_key() {
        let operator = Contains::build(
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
