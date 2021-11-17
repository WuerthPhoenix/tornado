use crate::accessor::Accessor;
use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::model::InternalEvent;
use std::cmp::Ordering;
use tornado_common_api::{partial_cmp_option_cow_value, Value};

const OPERATOR_NAME: &str = "le";

/// A matching matcher.operator that checks whether the first argument is less than the second one
#[derive(Debug)]
pub struct LessEqualThan {
    first: Accessor,
    second: Accessor,
}

impl LessEqualThan {
    pub fn build(first: Accessor, second: Accessor) -> Result<LessEqualThan, MatcherError> {
        Ok(LessEqualThan { first, second })
    }
}

impl Operator for LessEqualThan {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &InternalEvent, extracted_vars: Option<&Value>) -> bool {
        let cmp = partial_cmp_option_cow_value(&self.first.get(event, extracted_vars), || {
            self.second.get(event, extracted_vars)
        });
        cmp == Some(Ordering::Less) || cmp == Some(Ordering::Equal)
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
        let operator = LessEqualThan {
            first: AccessorBuilder::new().build("", &"".to_owned()).unwrap(),
            second: AccessorBuilder::new().build("", &"".to_owned()).unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"two".to_owned()).unwrap(),
        )
        .unwrap();

        let event = json!(Event::new("test_type"));

        assert_eq!("one", operator.first.get(&event, None).unwrap().as_ref());
        assert_eq!("two", operator.second.get(&event, None).unwrap().as_ref());
    }

    #[test]
    fn should_evaluate_to_true_if_equal_arguments() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&json!(event), None));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"two".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("one");

        assert!(operator.evaluate(&json!(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_greater() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"aaa".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("zzz");

        assert!(!operator.evaluate(&json!(event), None));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.type}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut payload = Map::new();
        payload.insert("type".to_owned(), Value::String("two".to_owned()));

        let event = Event::new_with_payload("one", payload);

        assert!(operator.evaluate(&json!(event), None));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.1}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.2}".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&json!(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_bool() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Bool(false));
        event.payload.insert("two".to_owned(), Value::Bool(false));

        assert!(operator.evaluate(&json!(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_number() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), json!(1.1));
        event.payload.insert("two".to_owned(), json!(1.1));

        assert!(operator.evaluate(&json!(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_values_of_type_number_and_less() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), json!(1));
        event.payload.insert("two".to_owned(), json!(1000000000.2));

        assert!(operator.evaluate(&json!(event), None));
    }

    #[test]
    fn should_evaluate_to_true_with_values_of_type_array() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "one".to_owned(),
            Value::Array(vec![
                json!(1),
                json!(-2000),
            ]),
        );
        event.payload.insert(
            "two".to_owned(),
            Value::Array(vec![json!(1), json!(-2)]),
        );

        assert!(operator.evaluate(&json!(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_equal_values_of_type_map() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut payload = Payload::new();
        payload.insert("one".to_owned(), json!(1.1));
        payload.insert("two".to_owned(), Value::Bool(true));
        payload.insert("three".to_owned(), Value::String("hello".to_owned()));

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Object(payload.clone()));
        event.payload.insert("two".to_owned(), Value::Object(payload.clone()));

        assert!(!operator.evaluate(&json!(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_different_type() {
        let operator = LessEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::String("1.2".to_owned()));
        event.payload.insert("two".to_owned(), json!(1.2));

        assert!(!operator.evaluate(&json!(event), None));
    }
}
