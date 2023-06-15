use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::{accessor::Accessor, model::InternalEvent};
use std::cmp::Ordering;
use tornado_common_api::partial_cmp_option_cow_value;

const OPERATOR_NAME: &str = "gt";

/// A matching matcher.operator that checks whether the first argument is greater than the second one
#[derive(Debug)]
pub struct GreaterThan {
    first: Accessor,
    second: Accessor,
}

impl GreaterThan {
    pub fn build(first: Accessor, second: Accessor) -> Result<GreaterThan, MatcherError> {
        Ok(GreaterThan { first, second })
    }
}

impl Operator for GreaterThan {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &InternalEvent) -> bool {
        let cmp = partial_cmp_option_cow_value(&self.first.get(event), || self.second.get(event));
        cmp == Some(Ordering::Greater)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::accessor::AccessorBuilder;
    use serde_json::json;
    use tornado_common_api::*;

    #[test]
    fn should_return_the_operator_name() {
        let operator = GreaterThan {
            first: AccessorBuilder::new().build("", "").unwrap(),
            second: AccessorBuilder::new().build("", "").unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator = GreaterThan::build(
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
    fn should_evaluate_to_false_if_equal_arguments() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "one").unwrap(),
            AccessorBuilder::new().build("", "one").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "one").unwrap(),
        )
        .unwrap();

        let event = Event::new("two");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_less() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "zzz").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.type}").unwrap(),
        )
        .unwrap();

        let mut payload = Map::new();
        payload.insert("type".to_owned(), Value::String("one".to_owned()));

        let event = Event::new_with_payload("two", payload);

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "${event.payload.1}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.2}").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_equal_values_of_type_bool() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Bool(false));
        event.payload.insert("two".to_owned(), Value::Bool(true));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_equal_values_of_type_number() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), json!(1.1));
        event.payload.insert("two".to_owned(), json!(1.1));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_values_of_type_number_and_greater() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), json!(1000));
        event.payload.insert("two".to_owned(), json!(1.2));

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_with_values_of_type_array() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Array(vec![json!(1000000001), json!(-2)]));
        event.payload.insert("two".to_owned(), Value::Array(vec![json!(1), json!(-2)]));

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_equal_values_of_type_map() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut payload = Payload::new();
        payload.insert("one".to_owned(), json!(1.1));
        payload.insert("two".to_owned(), Value::Bool(true));
        payload.insert("three".to_owned(), Value::String("hello".to_owned()));

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Object(payload.clone()));
        event.payload.insert("two".to_owned(), Value::Object(payload.clone()));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_different_type() {
        let operator = GreaterThan::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::String("1.2".to_owned()));
        event.payload.insert("two".to_owned(), json!(1.2));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }
}
