use crate::accessor::Accessor;
use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::model::InternalEvent;
use std::cmp::Ordering;
use std::collections::HashMap;
use tornado_common_api::{partial_cmp_option_cow_value, Value};

const OPERATOR_NAME: &str = "ge";

/// A matching matcher.operator that checks whether the first argument is equal or greater
/// than the second one
#[derive(Debug)]
pub struct GreaterEqualThan {
    first: Accessor,
    second: Accessor,
}

impl GreaterEqualThan {
    pub fn build(first: Accessor, second: Accessor) -> Result<GreaterEqualThan, MatcherError> {
        Ok(GreaterEqualThan { first, second })
    }
}

impl Operator for GreaterEqualThan {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(
        &self,
        event: &InternalEvent,
        extracted_vars: Option<&HashMap<String, Value>>,
    ) -> bool {
        let cmp = partial_cmp_option_cow_value(&self.first.get(event, extracted_vars), || {
            self.second.get(event, extracted_vars)
        });
        cmp == Some(Ordering::Greater) || cmp == Some(Ordering::Equal)
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
        let operator = GreaterEqualThan {
            first: AccessorBuilder::new().build("", &"".to_owned()).unwrap(),
            second: AccessorBuilder::new().build("", &"".to_owned()).unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"two".to_owned()).unwrap(),
        )
        .unwrap();

        let event = InternalEvent::new(Event::new("test_type"));

        assert_eq!("one", operator.first.get(&event, None).unwrap().as_ref());
        assert_eq!("two", operator.second.get(&event, None).unwrap().as_ref());
    }

    #[test]
    fn should_evaluate_to_true_if_equal_arguments() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("two");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_less() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"zzz".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.type}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut payload = HashMap::new();
        payload.insert("type".to_owned(), Value::Text("one".to_owned()));

        let event = Event::new_with_payload("two", payload);

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.1}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.2}".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_bool() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Bool(false));
        event.payload.insert("two".to_owned(), Value::Bool(false));

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_number() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Number(Number::Float(1.1)));
        event.payload.insert("two".to_owned(), Value::Number(Number::Float(1.1)));

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_values_of_type_number_and_greater() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Number(Number::PosInt(1000)));
        event.payload.insert("two".to_owned(), Value::Number(Number::Float(1.2)));

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_with_values_of_type_array() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "one".to_owned(),
            Value::Array(vec![
                Value::Number(Number::PosInt(1000000001)),
                Value::Number(Number::NegInt(-2)),
            ]),
        );
        event.payload.insert(
            "two".to_owned(),
            Value::Array(vec![Value::Number(Number::PosInt(1)), Value::Number(Number::NegInt(-2))]),
        );

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_equal_values_of_type_map() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut payload = Payload::new();
        payload.insert("one".to_owned(), Value::Number(Number::Float(1.1)));
        payload.insert("two".to_owned(), Value::Bool(true));
        payload.insert("three".to_owned(), Value::Text("hello".to_owned()));

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Map(payload.clone()));
        event.payload.insert("two".to_owned(), Value::Map(payload.clone()));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_different_type() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Text("1.2".to_owned()));
        event.payload.insert("two".to_owned(), Value::Number(Number::Float(1.2)));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_arrays_recursively() {
        let operator = GreaterEqualThan::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
            .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(),
                             Value::Array(vec![
                                Value::Text("id".to_owned()),
                                Value::Number(Number::PosInt(110))
                             ])
        );
        event.payload.insert("two".to_owned(),
        Value::Array(vec![
            Value::Text("id".to_owned()),
            Value::Number(Number::PosInt(10)),
            Value::Number(Number::PosInt(110))
        ]));

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }
}
