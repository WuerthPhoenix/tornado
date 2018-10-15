use rule::parser::RuleBuilderError;
use rule::Rule;

const RULE_NAME: &str = "equal";

/// A matching rule that evaluates whether two strings are equals.
#[derive(Debug)]
pub struct EqualRule {
    first_arg: String,
    second_arg: String,
}

impl EqualRule {
    pub fn build(args: &Vec<String>) -> Result<EqualRule, RuleBuilderError> {
        let expected = 2;
        if args.len() != expected {
            return Err(RuleBuilderError::WrongNumberOfArgumentsError {
                rule: RULE_NAME,
                expected: expected as u64,
                found: args.len() as u64,
            });
        }
        Ok(EqualRule {
            first_arg: args[0].clone(),
            second_arg: args[1].clone(),
        })
    }
}

impl Rule for EqualRule {
    fn name(&self) -> &str {
        RULE_NAME
    }

    fn evaluate(&self) -> bool {
        self.first_arg == self.second_arg
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_return_the_rule_name() {
        let rule = EqualRule {
            first_arg: "".to_owned(),
            second_arg: "".to_owned(),
        };
        assert_eq!(RULE_NAME, rule.name());
    }

    #[test]
    fn should_build_the_rule_with_expected_arguments() {
        let rule = EqualRule::build(&vec!["one".to_string(), "two".to_string()]).unwrap();
        assert_eq!("one".to_string(), rule.first_arg);
        assert_eq!("two".to_string(), rule.second_arg);
    }

    #[test]
    fn build_should_fail_if_not_enough_arguments() {
        let rule = EqualRule::build(&vec!["one".to_string()]);
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_fail_if_too_much_arguments() {
        let rule = EqualRule::build(&vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
        ]);
        assert!(rule.is_err());
    }

    #[test]
    fn should_evaluate_to_true_if_equal_arguments() {
        let rule = EqualRule::build(&vec!["one".to_string(), "one".to_string()]).unwrap();
        assert!(rule.evaluate());
    }

    #[test]
    fn should_evaluate_to_false_if_different_arguments() {
        let rule = EqualRule::build(&vec!["one".to_string(), "two".to_string()]).unwrap();
        assert!(!rule.evaluate());
    }
}
