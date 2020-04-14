pub mod id;

use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::config::MatcherConfig;
use crate::error::MatcherError;
use log::*;

/// A validator for a MatcherConfig
#[derive(Default)]
pub struct MatcherConfigValidator {
    id: id::IdValidator,
}

impl MatcherConfigValidator {
    pub fn new() -> MatcherConfigValidator {
        MatcherConfigValidator { id: id::IdValidator::new() }
    }

    pub fn validate(&self, config: &MatcherConfig) -> Result<(), MatcherError> {
        match config {
            MatcherConfig::Ruleset { name, rules } => self.validate_ruleset(name, rules),
            MatcherConfig::Filter { name, filter, nodes } => {
                self.validate_filter(name, filter, nodes)
            }
        }
    }

    /// Validates that a Filter has a valid name and triggers the validation recursively
    /// for all filter's nodes.
    fn validate_filter(
        &self,
        filter_name: &str,
        _filter: &Filter,
        nodes: &[MatcherConfig],
    ) -> Result<(), MatcherError> {
        debug!("MatcherConfigValidator validate_filter - validate filter [{}]", filter_name);

        self.id.validate_filter_name(filter_name)?;

        for node in nodes {
            self.validate(node)?;
        }

        Ok(())
    }

    /// Validates a set of Rules.
    /// In addition to the checks performed by the validate(rule) method,
    /// it verifies that rule names are unique.
    fn validate_ruleset(&self, ruleset_name: &str, rules: &[Rule]) -> Result<(), MatcherError> {
        debug!("MatcherConfigValidator validate_all - validate ruleset [{}]", ruleset_name);

        self.id.validate_ruleset_name(ruleset_name)?;

        let mut rule_names = vec![];

        for rule in rules {
            if rule.active {
                self.validate_rule(rule)?;
                MatcherConfigValidator::check_unique_name(&mut rule_names, &rule.name)?;
            }
        }

        Ok(())
    }

    /// Checks that a rule:
    /// - has a valid name
    /// - has valid extracted variable names
    /// - has valid action IDs
    fn validate_rule(&self, rule: &Rule) -> Result<(), MatcherError> {
        let rule_name = &rule.name;

        debug!("MatcherConfigValidator validate - Validating rule: [{}]", rule_name);
        self.id.validate_rule_name(rule_name)?;

        for var_name in rule.constraint.with.keys() {
            self.id.validate_extracted_var_name(var_name, rule_name)?
        }

        for action in &rule.actions {
            self.id.validate_action_id(&action.id, rule_name)?
        }

        Ok(())
    }

    fn check_unique_name(rule_names: &mut Vec<String>, name: &str) -> Result<(), MatcherError> {
        let name_string = name.to_owned();
        debug!(
            "MatcherConfigValidator - Validating uniqueness of name for rule: [{}]",
            &name_string
        );
        if rule_names.contains(&name_string) {
            return Err(MatcherError::NotUniqueRuleNameError { name: name_string });
        }
        rule_names.push(name_string);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::rule::{Action, Constraint, Extractor, ExtractorRegex, Operator};
    use crate::config::Defaultable;
    use std::collections::HashMap;
    use tornado_common_api::Value;

    #[test]
    fn should_fail_if_wrong_ruleset_name() {
        // Arrange
        let rule = new_rule(
            "rule_name",
            Operator::Equals {
                first: Value::Text("1".to_owned()),
                second: Value::Text("1".to_owned()),
            },
        );

        // Act
        let result = MatcherConfigValidator::new().validate_ruleset("", &vec![rule]);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_validate_correct_rule() {
        // Arrange
        let rule = new_rule(
            "rule_name",
            Operator::Equals {
                first: Value::Text("1".to_owned()),
                second: Value::Text("1".to_owned()),
            },
        );

        // Act
        let result = MatcherConfigValidator::new().validate_ruleset("ruleset", &vec![rule]);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn should_validate_correct_rules() {
        // Arrange
        let rule_1 = new_rule(
            "rule_name",
            Operator::Equals {
                first: Value::Text("1".to_owned()),
                second: Value::Text("1".to_owned()),
            },
        );

        let rule_2 = new_rule(
            "rule_name_2",
            Operator::Equals {
                first: Value::Text("1".to_owned()),
                second: Value::Text("1".to_owned()),
            },
        );

        // Act
        let result =
            MatcherConfigValidator::new().validate_ruleset("ruleset", &vec![rule_1, rule_2]);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn should_fail_validation_if_empty_name() {
        // Arrange
        let rule_1 = new_rule(
            "",
            Operator::Equals {
                first: Value::Text("1".to_owned()),
                second: Value::Text("1".to_owned()),
            },
        );

        // Act
        let result = MatcherConfigValidator::new().validate_ruleset("ruleset", &vec![rule_1]);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn build_should_fail_if_not_unique_name() {
        // Arrange
        let op = Operator::Equals {
            first: Value::Text("1".to_owned()),
            second: Value::Text("1".to_owned()),
        };
        let rule_1 = new_rule("rule_name", op.clone());
        let rule_2 = new_rule("rule_name", op.clone());

        // Act
        let matcher =
            MatcherConfigValidator::new().validate_ruleset("ruleset", &vec![rule_1, rule_2]);

        // Assert
        assert!(matcher.is_err());

        match matcher.err().unwrap() {
            MatcherError::NotUniqueRuleNameError { name } => assert_eq!("rule_name", name),
            _ => assert!(false),
        }
    }

    #[test]
    fn build_should_fail_if_empty_spaces_in_rule_name() {
        // Arrange
        let op = Operator::Equals {
            first: Value::Text("1".to_owned()),
            second: Value::Text("1".to_owned()),
        };
        let rule_1 = new_rule("rule name", op.clone());

        // Act
        let matcher = MatcherConfigValidator::new().validate_ruleset("ruleset", &vec![rule_1]);

        // Assert
        assert!(matcher.is_err());
    }

    #[test]
    fn build_should_fail_if_not_correct_name() {
        // Arrange
        let op = Operator::Equals {
            first: Value::Text("1".to_owned()),
            second: Value::Text("1".to_owned()),
        };
        let rule_1 = new_rule("rule.name", op.clone());

        // Act
        let matcher = MatcherConfigValidator::new().validate_ruleset("ruleset", &vec![rule_1]);

        // Assert
        assert!(matcher.is_err());
    }

    #[test]
    fn build_should_fail_if_not_correct_extracted_var_name() {
        // Arrange
        let op = Operator::Equals {
            first: Value::Text("1".to_owned()),
            second: Value::Text("1".to_owned()),
        };
        let mut rule_1 = new_rule("rule_name", op.clone());

        rule_1.constraint.with.insert(
            "var.with.dot".to_owned(),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
            },
        );

        // Act
        let matcher = MatcherConfigValidator::new().validate_ruleset("ruleset", &vec![rule_1]);

        // Assert
        assert!(matcher.is_err());
    }

    #[test]
    fn build_should_fail_if_not_correct_action_id() {
        // Arrange
        let op = Operator::Equals {
            first: Value::Text("1".to_owned()),
            second: Value::Text("1".to_owned()),
        };
        let mut rule_1 = new_rule("rule_name", op.clone());

        rule_1.actions.push(Action {
            id: "id.with.dot.and.question.mark?".to_owned(),
            payload: HashMap::new(),
        });

        // Act
        let matcher = MatcherConfigValidator::new().validate_ruleset("ruleset", &vec![rule_1]);

        // Assert
        assert!(matcher.is_err());
    }

    #[test]
    fn build_should_fail_if_wrong_filter_name() {
        // Arrange
        let filter =
            Filter { filter: Defaultable::Default {}, active: true, description: "".to_owned() };

        // Act
        let matcher = MatcherConfigValidator::new().validate_filter(
            "wrong.because.of.dots",
            &filter,
            &vec![],
        );

        // Assert
        assert!(matcher.is_err());
    }

    #[test]
    fn should_validate_filter_name() {
        // Arrange
        let filter =
            Filter { filter: Defaultable::Default {}, active: true, description: "".to_owned() };

        // Act
        let matcher = MatcherConfigValidator::new().validate_filter("good_name", &filter, &vec![]);

        // Assert
        assert!(matcher.is_ok());
    }

    #[test]
    fn build_should_fail_if_wrong_node_name() {
        // Arrange
        let filter =
            Filter { filter: Defaultable::Default {}, active: true, description: "".to_owned() };

        let rules = MatcherConfig::Ruleset { name: "wrong.name!".to_owned(), rules: vec![] };

        // Act
        let matcher =
            MatcherConfigValidator::new().validate_filter("good_names", &filter, &vec![rules]);

        // Assert
        assert!(matcher.is_err());
    }

    #[test]
    fn should_validate_node_name() {
        // Arrange
        let filter =
            Filter { filter: Defaultable::Default {}, active: true, description: "".to_owned() };

        let rules = MatcherConfig::Ruleset { name: "good_name".to_owned(), rules: vec![] };

        // Act
        let matcher =
            MatcherConfigValidator::new().validate_filter("good_names", &filter, &vec![rules]);

        // Assert
        assert!(matcher.is_ok());
    }

    #[test]
    fn should_validate_a_config_recursively() {
        // Arrange
        let filter1 =
            Filter { filter: Defaultable::Default {}, active: true, description: "".to_owned() };

        let filter2 = filter1.clone();
        let rule_1 = new_rule("rule_name", None);

        let config = MatcherConfig::Filter {
            name: "good_name".to_owned(),
            filter: filter1,
            nodes: vec![
                MatcherConfig::Filter {
                    name: "good_name".to_owned(),
                    filter: filter2,
                    nodes: vec![],
                },
                MatcherConfig::Ruleset {
                    name: "good_ruleset_name".to_owned(),
                    rules: vec![rule_1],
                },
            ],
        };

        // Act
        let matcher = MatcherConfigValidator::new().validate(&config);

        // Assert
        assert!(matcher.is_ok());
    }

    #[test]
    fn should_validate_a_config_recursively_and_fail_if_wrong_inner_rule_name() {
        // Arrange
        let filter1 =
            Filter { filter: Defaultable::Default {}, active: true, description: "".to_owned() };

        let filter2 = filter1.clone();
        let rule_1 = new_rule("rule.name!", None);

        let config = MatcherConfig::Filter {
            name: "good_name".to_owned(),
            filter: filter1,
            nodes: vec![
                MatcherConfig::Filter {
                    name: "good_name".to_owned(),
                    filter: filter2,
                    nodes: vec![],
                },
                MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![rule_1] },
            ],
        };

        // Act
        let matcher = MatcherConfigValidator::new().validate(&config);

        // Assert
        assert!(matcher.is_err());
    }

    fn new_rule<O: Into<Option<Operator>>>(name: &str, operator: O) -> Rule {
        let constraint = Constraint { where_operator: operator.into(), with: HashMap::new() };

        Rule {
            name: name.to_owned(),
            do_continue: true,
            active: true,
            actions: vec![],
            description: "".to_owned(),
            constraint,
        }
    }
}
