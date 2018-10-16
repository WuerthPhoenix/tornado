use error::MatcherError;
use operator::parser::OperatorBuilder;
use operator::Operator;
use tornado_common::Event;

const OPERATOR_NAME: &str = "and";

/// A matching operator that evaluates whether a list of children rules are all verified.
#[derive(Debug)]
pub struct And {
    rules: Vec<Box<Operator>>,
}

impl And {
    pub fn build(args: &Vec<String>, builder: &OperatorBuilder) -> Result<And, MatcherError> {
        let mut rules = vec![];
        for entry in args {
            let args = builder.parse(entry.to_owned())?;
            let rule = builder.build(&args)?;
            rules.push(rule)
        }
        Ok(And { rules })
    }
}

impl Operator for And {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, event: &Event) -> bool {
        for rule in &self.rules {
            if !rule.evaluate(event) {
                return false;
            }
        }
        return true;
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn should_return_the_rule_name() {
        let rule = And { rules: vec![] };
        assert_eq!(OPERATOR_NAME, rule.name());
    }

    #[test]
    fn should_build_the_and_with_expected_arguments() {
        let rule = And::build(&vec!["[=,1,2]".to_string()], &OperatorBuilder::new()).unwrap();
        assert_eq!(1, rule.rules.len());
        assert_eq!("equal", rule.rules[0].name());
    }

    #[test]
    fn should_build_the_and_with_no_arguments() {
        let rule = And::build(&vec![], &OperatorBuilder::new()).unwrap();
        assert_eq!(0, rule.rules.len());
    }

    #[test]
    fn build_should_fail_if_wrong_nested_rule() {
        let rule = And::build(&vec!["WRONG_RULE_NAME".to_owned()], &OperatorBuilder::new());
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_be_recursive() {
        let rule = And::build(
            &vec!["[=,1,2]".to_string(), "[or,[=,3,4]]".to_string()],
            &OperatorBuilder::new(),
        ).unwrap();
        assert_eq!("and", rule.name());
        assert_eq!(2, rule.rules.len());
        assert_eq!("equal", rule.rules[0].name());
        assert_eq!("or", rule.rules[1].name());

        println!("{:?}", rule.rules[1]);

        assert!(
            format!("{:?}", rule.rules[1])
                .contains(r#"Equal { first_arg: ConstantAccessor { value: "3" }, second_arg: ConstantAccessor { value: "4" } }"#)
        )
    }

    #[test]
    fn should_evaluate_to_true_if_no_children() {
        let rule = And::build(&vec![], &OperatorBuilder::new()).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_true_if_all_children_match() {
        let rule = And::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[=,3,3]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_false_if_not_all_children_match() {
        let rule = And::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[=,3,3]".to_string(),
                "[=,4,1]".to_string(),
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_true_if_all_children_match_recursively() {
        let rule = And::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[and,[=,3,3], [and,[=,6,6]]]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_false_if_not_all_children_match_recursively() {
        let rule = And::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[and,[=,3,3], [and,[=,5,6]]]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively() {
        let rule = And::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[and,[=,3,3], [and,[=,${event.type},type]]]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "type".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_using_accessors_recursively_and_return_false() {
        let rule = And::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[and,[=,3,3], [and,[=,${event.type},type1]]]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &OperatorBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "type".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

}
