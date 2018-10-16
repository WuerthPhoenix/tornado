use error::MatcherError;
use rule::parser::{RuleBuilder};
use rule::Rule;
use tornado_common::Event;

const RULE_NAME: &str = "and";

/// A matching rule that evaluates whether a list of children rules are all verified.
#[derive(Debug)]
pub struct AndRule {
    rules: Vec<Box<Rule>>,
}

impl AndRule {
    pub fn build(args: &Vec<String>, builder: &RuleBuilder) -> Result<AndRule, MatcherError> {
        let mut rules = vec![];
        for entry in args {
            let args = builder.parse(entry.to_owned())?;
            let rule = builder.build(&args)?;
            rules.push(rule)
        }
        Ok(AndRule { rules })
    }
}

impl Rule for AndRule {
    fn name(&self) -> &str {
        RULE_NAME
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
        let rule = AndRule { rules: vec![] };
        assert_eq!(RULE_NAME, rule.name());
    }

    #[test]
    fn should_build_the_and_with_expected_arguments() {
        let rule = AndRule::build(&vec!["[=,1,2]".to_string()], &RuleBuilder::new()).unwrap();
        assert_eq!(1, rule.rules.len());
        assert_eq!("equal", rule.rules[0].name());
    }

    #[test]
    fn should_build_the_and_with_no_arguments() {
        let rule = AndRule::build(&vec![], &RuleBuilder::new()).unwrap();
        assert_eq!(0, rule.rules.len());
    }

    #[test]
    fn build_should_fail_if_wrong_nested_rule() {
        let rule = AndRule::build(&vec!["WRONG_RULE_NAME".to_owned()], &RuleBuilder::new());
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_be_recursive() {
        let rule = AndRule::build(
            &vec!["[=,1,2]".to_string(), "[or,[=,3,4]]".to_string()],
            &RuleBuilder::new(),
        ).unwrap();
        assert_eq!("and", rule.name());
        assert_eq!(2, rule.rules.len());
        assert_eq!("equal", rule.rules[0].name());
        assert_eq!("or", rule.rules[1].name());

        println!("{:?}", rule.rules[1]);

        assert!(
            format!("{:?}", rule.rules[1])
                .contains(r#"EqualRule { first_arg: ConstantAccessor { value: "3" }, second_arg: ConstantAccessor { value: "4" } }"#)
        )
    }

    #[test]
    fn should_evaluate_to_true_if_no_children() {
        let rule = AndRule::build(&vec![], &RuleBuilder::new()).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "".to_owned(),
            created_ts: 0,
        };

        assert!(rule.evaluate(&event));
    }

    #[test]
    fn should_evaluate_to_true_if_all_children_match() {
        let rule = AndRule::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[=,3,3]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &RuleBuilder::new(),
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
        let rule = AndRule::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[=,3,3]".to_string(),
                "[=,4,1]".to_string(),
            ],
            &RuleBuilder::new(),
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
        let rule = AndRule::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[and,[=,3,3], [and,[=,6,6]]]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &RuleBuilder::new(),
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
        let rule = AndRule::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[and,[=,3,3], [and,[=,5,6]]]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &RuleBuilder::new(),
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
        let rule = AndRule::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[and,[=,3,3], [and,[=,${event.type},type]]]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &RuleBuilder::new(),
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
        let rule = AndRule::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[and,[=,3,3], [and,[=,${event.type},type1]]]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &RuleBuilder::new(),
        ).unwrap();

        let event = Event {
            payload: HashMap::new(),
            event_type: "type".to_owned(),
            created_ts: 0,
        };

        assert!(!rule.evaluate(&event));
    }

}
