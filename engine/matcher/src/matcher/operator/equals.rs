use crate::accessor::Accessor;
use crate::error::MatcherError;
use crate::matcher::operator::Operator;
use crate::model::InternalEvent;

const OPERATOR_NAME: &str = "equals";

/// A matching matcher.operator that checks whether two values are equal
#[derive(Debug)]
pub struct Equals {
    first_arg: Accessor,
    second_arg: Accessor,
}

impl Equals {
    pub fn build(first_arg: Accessor, second_arg: Accessor) -> Result<Equals, MatcherError> {
        Ok(Equals { first_arg, second_arg })
    }
}

impl Operator for Equals {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &InternalEvent) -> bool {
        let first = self.first_arg.get(event);
        let second = self.second_arg.get(event);
        first == second
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
        let operator = Equals {
            first_arg: AccessorBuilder::new().build("", "").unwrap(),
            second_arg: AccessorBuilder::new().build("", "").unwrap(),
        };
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_build_the_operator_with_expected_arguments() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "one").unwrap(),
            AccessorBuilder::new().build("", "two").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert_eq!(
            "one",
            operator.first_arg.get(&(&json!(event), &mut Value::Null).into()).unwrap().as_ref()
        );
        assert_eq!(
            "two",
            operator.second_arg.get(&(&json!(event), &mut Value::Null).into()).unwrap().as_ref()
        );
    }

    #[test]
    fn should_evaluate_to_true_if_equal_arguments() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "one").unwrap(),
            AccessorBuilder::new().build("", "one").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "test_type").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_different_arguments() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "wrong_test_type").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.type}").unwrap(),
        )
        .unwrap();

        let mut payload = Map::new();
        payload.insert("type".to_owned(), Value::String("type".to_owned()));

        let event = Event::new_with_payload("type", payload);

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_return_true_if_both_fields_do_not_exist() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.payload.1}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.2}").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_return_false_if_one_field_does_not_exist() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.2}").unwrap(),
        )
        .unwrap();

        let event = Event::new("test_type");

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_bool() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Bool(false));
        event.payload.insert("two".to_owned(), Value::Bool(false));

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_type_bool_but_not_equal() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Bool(true));
        event.payload.insert("two".to_owned(), Value::Bool(false));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_number() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), json!(1.1));
        event.payload.insert("two".to_owned(), json!(1.1));

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_type_number_but_not_equal() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), json!(1.1));
        event.payload.insert("two".to_owned(), json!(1.2));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_array() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Array(vec![json!(1.1), json!(-2)]));
        event.payload.insert("two".to_owned(), Value::Array(vec![json!(1.1), json!(-2)]));

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_type_array_but_different() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Array(vec![json!(1.1), json!(2.2)]));
        event.payload.insert("two".to_owned(), Value::Array(vec![json!(1.1)]));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_true_if_equal_values_of_type_map() {
        let operator = Equals::build(
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

        assert!(operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_type_map_but_different() {
        let operator = Equals::build(
            AccessorBuilder::new().build("", "${event.payload.one}").unwrap(),
            AccessorBuilder::new().build("", "${event.payload.two}").unwrap(),
        )
        .unwrap();

        let mut payload = Payload::new();
        payload.insert("one".to_owned(), json!(1.1));
        payload.insert("two".to_owned(), Value::Bool(true));

        let mut event = Event::new("test_type");
        event.payload.insert("one".to_owned(), Value::Object(payload.clone()));

        payload.insert("three".to_owned(), Value::String("hello".to_owned()));
        event.payload.insert("two".to_owned(), Value::Object(payload.clone()));

        assert!(!operator.evaluate(&(&json!(event), &mut Value::Null).into()));
    }

    #[test]
    fn should_evaluate_to_false_if_values_of_different_type() {
        let operator = Equals::build(
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
