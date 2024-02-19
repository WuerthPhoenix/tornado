use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::{accessor::Accessor, model::InternalEvent};
use log::*;
use tornado_common_api::{cow_to_str, ValueExt};

const OPERATOR_NAME: &str = "equalsIgnoreCase";

/// A matching matcher.operator that evaluates whether the two strings passed as arguments
/// are equal to each other, in a case-insensitive way.
/// If one or both the arguments are not strings, the operator will evaluate to false
#[derive(Debug)]
pub struct EqualsIgnoreCase {
    first: Accessor,
    second: Accessor,
}

impl EqualsIgnoreCase {
    pub fn build(first: Accessor, second: Accessor) -> Result<EqualsIgnoreCase, MatcherError> {
        Ok(EqualsIgnoreCase { first, second })
    }
}

impl Operator for EqualsIgnoreCase {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &InternalEvent) -> bool {
        match self.first.get(event) {
            Some(first_value) => match first_value.get_text() {
                Some(first) => {
                    let option_substring = self.second.get(event);
                    match cow_to_str(&option_substring) {
                        Some(substring) => first.to_lowercase().eq(&substring.to_lowercase()),
                        None => {
                            trace!("EqualsIgnoreCase - The second argument must be of type Value::Text, found instead {:#?}, evaluating to false", option_substring);
                            false
                        }
                    }
                }
                None => {
                    trace!("EqualsIgnoreCase - The first argument must be of type Value::Text, found instead {:#?}, evaluating to false", first_value);
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
    use serde_json::json;
    use tornado_common_api::*;

    #[test]
    fn should_return_the_operator_name() {
        let operator = EqualsIgnoreCase {
            first: AccessorBuilder::new().build("", "").unwrap(),
            second: AccessorBuilder::new().build("", "").unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator = EqualsIgnoreCase::build(
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
    fn should_evaluate_to_false_if_first_arg_is_not_string() {
        let operator = EqualsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.payload.value}").unwrap(),
            AccessorBuilder::new().build("", "9").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("TEst_type");
        event.payload.insert("value".to_owned(), json!(9));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_second_arg_is_not_string() {
        let operator = EqualsIgnoreCase::build(
            AccessorBuilder::new().build("", "9").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.value}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("TEst_type");
        event.payload.insert("value".to_owned(), json!(9));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_args_are_equal_but_not_string() {
        let operator = EqualsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), json!(9));
        event.payload.insert("two".to_owned(), json!(9));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_text_equals_with_diff_case() {
        let operator = EqualsIgnoreCase::build(
            AccessorBuilder::new().build("", "one tWo_THREE").unwrap(),
            AccessorBuilder::new().build("", "ONE tWo_three").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_texts_are_equal_numbers() {
        let operator = EqualsIgnoreCase::build(
            AccessorBuilder::new().build("", "12").unwrap(),
            AccessorBuilder::new().build("", "12").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_text_is_not_equal() {
        let operator = EqualsIgnoreCase::build(
            AccessorBuilder::new().build("", "one two three").unwrap(),
            AccessorBuilder::new().build("", "one two").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = EqualsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "test_TYPE").unwrap(),
        )
        .unwrap();

        let event = Event::new("tEST_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = EqualsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.type}").unwrap(),
        )
        .unwrap();

        let mut payload = Map::new();
        payload.insert("type".to_owned(), Value::String("tyPe".to_owned()));

        let event = Event::new_with_payload("Type", payload);

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = EqualsIgnoreCase::build(
            AccessorBuilder::new().build("", "${event.payload.1}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.2}").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }
    #[test]
    fn should_evaluate_to_false_if_array_contains_the_second_arg() {
        let operator = EqualsIgnoreCase::build(
            AccessorBuilder::new()
                .build_from_value("", &Value::Array(vec![Value::String("one".to_owned())]))
                .unwrap(),
            AccessorBuilder::new().build_from_value("", &Value::String("one".to_owned())).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_map_contains_the_second_arg_as_key() {
        let operator = EqualsIgnoreCase::build(
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

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }
}
