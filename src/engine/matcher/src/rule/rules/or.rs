use rule::parser::{RuleBuilder, RuleBuilderError};
use rule::Rule;

const RULE_NAME: &str = "or";

/// A matching rule that evaluates whether at list one children on a list of rules matches.
#[derive(Debug)]
pub struct OrRule {
    rules: Vec<Box<Rule>>,
}

impl OrRule {
    pub fn build(args: &Vec<String>, builder: &RuleBuilder) -> Result<OrRule, RuleBuilderError> {
        let mut rules = vec![];
        for entry in args {
            let args = builder.parse(entry.to_owned())?;
            let rule = builder.build(&args)?;
            rules.push(rule)
        }
        Ok(OrRule { rules })
    }
}

impl Rule for OrRule {
    fn name(&self) -> &str {
        RULE_NAME
    }

    fn evaluate(&self) -> bool {
        for rule in &self.rules {
            if rule.evaluate() {
                return true;
            }
        }
        return false;
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_return_the_rule_name() {
        let rule = OrRule { rules: vec![] };
        assert_eq!(RULE_NAME, rule.name());
    }

    #[test]
    fn should_build_the_or_with_expected_arguments() {
        let rule = OrRule::build(&vec!["[=,1,2]".to_string()], &RuleBuilder::new()).unwrap();
        assert_eq!(1, rule.rules.len());
        assert_eq!("equal", rule.rules[0].name());
    }

    #[test]
    fn should_build_the_or_with_no_arguments() {
        let rule = OrRule::build(&vec![], &RuleBuilder::new()).unwrap();
        assert_eq!(0, rule.rules.len());
    }

    #[test]
    fn build_should_fail_if_wrong_nested_rule() {
        let rule = OrRule::build(&vec!["WRONG_RULE_NAME".to_owned()], &RuleBuilder::new());
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_be_recursive() {
        let rule = OrRule::build(
            &vec!["[=,1,2]".to_string(), "[and,[=,3,4]]".to_string()],
            &RuleBuilder::new(),
        ).unwrap();
        assert_eq!("or", rule.name());
        assert_eq!(2, rule.rules.len());
        assert_eq!("equal", rule.rules[0].name());
        assert_eq!("and", rule.rules[1].name());

        println!("{:?}", rule.rules[1]);

        assert!(
            format!("{:?}", rule.rules[1])
                .contains(r#"EqualRule { first_arg: "3", second_arg: "4" }"#)
        )
    }

    #[test]
    fn should_evaluate_to_false_if_no_children() {
        let rule = OrRule::build(&vec![], &RuleBuilder::new()).unwrap();
        assert!(!rule.evaluate());
    }

    #[test]
    fn should_evaluate_to_true_if_all_children_match() {
        let rule = OrRule::build(
            &vec![
                "[=,1,1]".to_string(),
                "[=,2,2]".to_string(),
                "[=,3,3]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &RuleBuilder::new(),
        ).unwrap();
        assert!(rule.evaluate());
    }

    #[test]
    fn should_evaluate_to_true_if_at_least_a_children_matches() {
        let rule = OrRule::build(
            &vec![
                "[=,1,4]".to_string(),
                "[=,2,4]".to_string(),
                "[=,3,4]".to_string(),
                "[=,4,4]".to_string(),
            ],
            &RuleBuilder::new(),
        ).unwrap();
        assert!(rule.evaluate());
    }

    #[test]
    fn should_evaluate_to_false_if_no_children_match() {
        let rule = OrRule::build(
            &vec![
                "[=,1,5]".to_string(),
                "[=,2,5]".to_string(),
                "[=,3,5]".to_string(),
                "[=,4,5]".to_string(),
            ],
            &RuleBuilder::new(),
        ).unwrap();
        assert!(!rule.evaluate());
    }

    #[test]
    fn should_evaluate_to_true_if_at_least_a_children_matches_recursively() {
        let rule = OrRule::build(
            &vec![
                "[=,1,5]".to_string(),
                "[=,2,5]".to_string(),
                "[or,[=,3,5], [and,[=,5,5]]]".to_string(),
                "[=,4,5]".to_string(),
            ],
            &RuleBuilder::new(),
        ).unwrap();
        assert!(rule.evaluate());
    }

    #[test]
    fn should_evaluate_to_false_if_no_children_match_recursively() {
        let rule = OrRule::build(
            &vec![
                "[=,1,6]".to_string(),
                "[=,2,6]".to_string(),
                "[and,[=,3,6], [and,[=,5,6]]]".to_string(),
                "[=,4,6]".to_string(),
            ],
            &RuleBuilder::new(),
        ).unwrap();
        assert!(!rule.evaluate());
    }

}
