use accessor::Accessor;
use error::MatcherError;
use model::ProcessedEvent;
use operator::Operator;

const OPERATOR_NAME: &str = "equal";

/// A matching operator that evaluates whether two strings are equal.
#[derive(Debug)]
pub struct Equal {
    first_arg: Accessor,
    second_arg: Accessor,
}

impl Equal {
    pub fn build(first_arg: Accessor, second_arg: Accessor) -> Result<Equal, MatcherError> {
        Ok(Equal {
            first_arg,
            second_arg,
        })
    }
}

impl Operator for Equal {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &ProcessedEvent) -> bool {
        let first = self.first_arg.get(event);
        let second = self.second_arg.get(event);
        first.is_some() && second.is_some() && (first == second)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use accessor::AccessorBuilder;
    use std::collections::HashMap;
    use tornado_common_api::Event;

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
        ).unwrap();

        let event = ProcessedEvent::new(Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        });

        assert_eq!("one", operator.first_arg.get(&event).unwrap());
        assert_eq!("two", operator.second_arg.get(&event).unwrap());
    }

    #[test]
    fn should_evaluate_to_true_if_equal_arguments() {
        let operator = Equal::build(
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
            AccessorBuilder::new().build("", &"one".to_owned()).unwrap(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_using_accessors() {
        let operator = Equal::build(
            AccessorBuilder::new()
                .build("", &"${event.type}".to_owned())
                .unwrap(),
            AccessorBuilder::new()
                .build("", &"test_type".to_owned())
                .unwrap(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_evaluate_to_false_if_different_arguments() {
        let operator = Equal::build(
            AccessorBuilder::new()
                .build("", &"${event.type}".to_owned())
                .unwrap(),
            AccessorBuilder::new()
                .build("", &"wrong_test_type".to_owned())
                .unwrap(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "test_type".to_owned(),
            created_ts: 0,
        };

        assert!(!operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_compare_event_fields() {
        let operator = Equal::build(
            AccessorBuilder::new()
                .build("", &"${event.type}".to_owned())
                .unwrap(),
            AccessorBuilder::new()
                .build("", &"${event.payload.type}".to_owned())
                .unwrap(),
        ).unwrap();

        let mut payload = HashMap::new();
        payload.insert("type".to_owned(), "type".to_owned());

        let event = Event {
            payload,
            event_type: "type".to_owned(),
            created_ts: 0,
        };

        assert!(operator.evaluate(&ProcessedEvent::new(event)));
    }

    #[test]
    fn should_return_false_if_fields_do_not_exist() {
        let operator = Equal::build(
            AccessorBuilder::new()
                .build("", &"${event.payload.1}".to_owned())
                .unwrap(),
            AccessorBuilder::new()
                .build("", &"${event.payload.2}".to_owned())
                .unwrap(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "type".to_owned(),
            created_ts: 0,
        };

        assert!(!operator.evaluate(&ProcessedEvent::new(event)));
    }

}
