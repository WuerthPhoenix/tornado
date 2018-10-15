use regex::Regex;
use rule::parser::RuleBuilderError;
use rule::Rule;

const RULE_NAME: &str = "regex";

/// A matching rule that evaluates whether a string matches a regex.
#[derive(Debug)]
pub struct RegexRule {
    regex: Regex,
    target: String,
}

impl RegexRule {
    pub fn build(args: &Vec<String>) -> Result<RegexRule, RuleBuilderError> {
        let expected = 2;
        if args.len() != expected {
            return Err(RuleBuilderError::WrongNumberOfArgumentsError {
                rule: RULE_NAME,
                expected: expected as u64,
                found: args.len() as u64,
            });
        }
        let regex_string = args[0].clone();
        let regex_str = regex_string.as_str();
        let target = args[1].clone();
        let regex =
            Regex::new(regex_str).map_err(|e| RuleBuilderError::OperatorBuildFailError {
                message: format!("Cannot parse regex [{}]", regex_str),
                cause: e.to_string(),
            })?;

        Ok(RegexRule { target, regex })
    }
}

impl Rule for RegexRule {
    fn name(&self) -> &str {
        RULE_NAME
    }

    fn evaluate(&self) -> bool {
        self.regex.is_match(self.target.as_str())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_return_the_rule_name() {
        let rule = RegexRule {
            regex: Regex::new("").unwrap(),
            target: "".to_owned(),
        };
        assert_eq!(RULE_NAME, rule.name());
    }

    #[test]
    fn should_build_the_rule_with_expected_arguments() {
        let rule = RegexRule::build(&vec!["one".to_string(), "two".to_string()]).unwrap();
        assert_eq!("one".to_string(), rule.regex.to_string());
        assert_eq!("two".to_string(), rule.target);
    }

    #[test]
    fn build_should_fail_if_not_enough_arguments() {
        let rule = RegexRule::build(&vec!["one".to_string()]);
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_fail_if_too_much_arguments() {
        let rule = RegexRule::build(&vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
        ]);
        assert!(rule.is_err());
    }

    #[test]
    fn build_should_fail_if_invalid_regex() {
        let rule = RegexRule::build(&vec!["[".to_string(), "two".to_string()]);
        assert!(rule.is_err());
    }

    #[test]
    fn should_evaluate_to_true_if_it_matches_the_regex() {
        let rule = RegexRule::build(&vec!["[a-fA-F0-9]".to_string(), "f".to_string()]).unwrap();
        assert!(rule.evaluate());
    }

    #[test]
    fn should_evaluate_to_false_if_it_does_not_match_the_regex() {
        let rule = RegexRule::build(&vec!["[a-fA-F0-9]".to_string(), "g".to_string()]).unwrap();
        assert!(!rule.evaluate());
    }
}
