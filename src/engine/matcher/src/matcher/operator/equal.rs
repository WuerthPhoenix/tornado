use crate::accessor::Accessor;
use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::model::InternalEvent;
use std::collections::HashMap;
use tornado_common_api::Value;

const OPERATOR_NAME: &str = "equal";

/// A matching matcher.operator that checks whether two strings are equal
#[derive(Debug)]
pub struct Equal {
    first_arg: Accessor,
    second_arg: Accessor,
}

impl Equal {
    pub fn build(first_arg: Accessor, second_arg: Accessor) -> Result<Equal, MatcherError> {
        Ok(Equal { first_arg, second_arg })
    }
}

impl Operator for Equal {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(
        &self,
        event: &InternalEvent,
        extracted_vars: Option<&HashMap<String, Value>>,
    ) -> bool {
        let first = self.first_arg.get(event, extracted_vars);
        if first.is_some() {
            let second = self.second_arg.get(event, extracted_vars);
            second.is_some() && (first == second)
        } else {
            false
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
        let operator = Equal {
            first_arg: AccessorBuilder::new().build("", &"".to_owned()).unwrap(),
            second_arg: AccessorBuilder::new().build("", &"".to_owned()).unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"two".to_owned()).unwrap(),
        )
        .unwrap();

        let event = InternalEvent::new(Event::new("test_type"));

        assert_eq!("one", operator.first_arg.get(&event, None).unwrap().as_ref());
        assert_eq!("two", operator.second_arg.get(&event, None).unwrap().as_ref());
    }

    #[test]
    fn should_evaluate_to_true_if_equal_arguments() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"test_type".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_different_arguments() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"wrong_test_type".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"${event.type}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.type}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut payload = HashMap::new();
        payload.insert("type".to_owned(), Value::Text("type".to_owned()));

        let event = Event::new_with_payload("type", payload);

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"${event.payload.1}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.2}".to_owned()).unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_bool() {
        let operator = Equal::build(
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
    fn should_evaluate_to_false_if_values_of_type_bool_but_not_equal() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Bool(true));
        event.payload.insert("two".to_owned(), Value::Bool(false));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_number() {
        let operator = Equal::build(
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
    fn should_evaluate_to_false_if_values_of_type_number_but_not_equal() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Number(Number::Float(1.1)));
        event.payload.insert("two".to_owned(), Value::Number(Number::Float(1.2)));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_array() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "one".to_owned(),
            Value::Array(vec![
                Value::Number(Number::Float(1.1)),
                Value::Number(Number::NegInt(-2)),
            ]),
        );
        event.payload.insert(
            "two".to_owned(),
            Value::Array(vec![
                Value::Number(Number::Float(1.1)),
                Value::Number(Number::NegInt(-2)),
            ]),
        );

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_type_array_but_different() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert(
            "one".to_owned(),
            Value::Array(vec![
                Value::Number(Number::Float(1.1)),
                Value::Number(Number::Float(2.2)),
            ]),
        );
        event
            .payload
            .insert("two".to_owned(), Value::Array(vec![Value::Number(Number::Float(1.1))]));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_map() {
        let operator = Equal::build(
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

        assert!(operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_type_map_but_different() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut payload = Payload::new();
        payload.insert("one".to_owned(), Value::Number(Number::Float(1.1)));
        payload.insert("two".to_owned(), Value::Bool(true));

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Map(payload.clone()));

        payload.insert("three".to_owned(), Value::Text("hello".to_owned()));
        event.payload.insert("two".to_owned(), Value::Map(payload.clone()));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_different_type() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"${event.payload.one}".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"${event.payload.two}".to_owned()).unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Text("1.2".to_owned()));
        event.payload.insert("two".to_owned(), Value::Number(Number::Float(1.2)));

        assert!(!operator.evaluate(&InternalEvent::new(event), None));
    }
}
