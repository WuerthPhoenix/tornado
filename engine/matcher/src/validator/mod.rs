pub mod id;

use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::config::{filter, MatcherConfig};
use crate::error::MatcherError;
use log::*;
use std::fmt::{Display, Formatter};

/// A validator for a MatcherConfig
#[derive(Default)]
pub struct MatcherConfigValidator {
    id: id::IdValidator,
}

pub enum NodePath<'parent> {
    Root,
    Parent { name: &'parent str, parent: &'parent NodePath<'parent>, is_iterator: bool },
}

impl Display for NodePath<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodePath::Root => f.write_str("root"),
            NodePath::Parent { name, parent, .. } => f.write_fmt(format_args!("{parent}.{name}")),
        }
    }
}

impl NodePath<'_> {
    pub fn get_iterator_ancestor(&self) -> Option<&NodePath> {
        match self {
            NodePath::Root => None,
            NodePath::Parent { is_iterator: true, .. } => Some(self),
            NodePath::Parent { is_iterator: false, parent, .. } => parent.get_iterator_ancestor(),
        }
    }
}

impl MatcherConfigValidator {
    pub fn new() -> MatcherConfigValidator {
        MatcherConfigValidator { id: id::IdValidator::new() }
    }

    pub fn validate(&self, config: &MatcherConfig) -> Result<(), MatcherError> {
        self.validate_inner(config, &NodePath::Root)
    }

    fn validate_inner(
        &self,
        config: &MatcherConfig,
        parent: &NodePath,
    ) -> Result<(), MatcherError> {
        match config {
            MatcherConfig::Ruleset { name, rules } => self.validate_ruleset(name, rules, parent),
            MatcherConfig::Filter { name, filter, nodes } => {
                self.validate_filter(name, filter, nodes, parent)
            }
            MatcherConfig::Iterator { name, target, nodes } => {
                self.validate_iterator(name, target, nodes, parent)
            }
        }
    }

    /// Validates that a Filter has a valid name and triggers the validation recursively
    /// for all filter's nodes.
    fn validate_filter(
        &self,
        name: &str,
        _filter: &Filter,
        nodes: &[MatcherConfig],
        parent: &NodePath,
    ) -> Result<(), MatcherError> {
        debug!("MatcherConfigValidator validate_filter - validate filter [{}]", name);
        let node_path = NodePath::Parent { name, parent, is_iterator: false };
        self.id.validate_filter_name(parent, name)?;

        for node in nodes {
            self.validate_inner(node, &node_path)?;
        }

        Ok(())
    }

    fn validate_iterator(
        &self,
        name: &str,
        _target: &str,
        nodes: &[MatcherConfig],
        parent: &NodePath,
    ) -> Result<(), MatcherError> {
        debug!("MatcherConfigValidator validate_iterator - validate iterator [{}]", name);
        let node_path = NodePath::Parent { name, parent, is_iterator: true };
        self.id.validate_iterator_name(parent, name)?;

        if let Some(ancestor) = parent.get_iterator_ancestor() {
            return Err(MatcherError::ConfigurationError {
                message: format!("Iterator in path [{node_path}] has already a iterator as its ancestor on [{ancestor}]"),
            });
        }

        for node in nodes {
            self.validate_inner(node, &node_path)?;
        }

        Ok(())
    }

    /// Validates a set of Rules.
    /// In addition to the checks performed by the validate(rule) method,
    /// it verifies that rule names are unique.
    fn validate_ruleset(
        &self,
        name: &str,
        rules: &[Rule],
        parent: &NodePath,
    ) -> Result<(), MatcherError> {
        debug!("MatcherConfigValidator validate_all - validate ruleset [{}]", name);
        let node_path = NodePath::Parent { name, parent, is_iterator: false };

        self.id.validate_ruleset_name(parent, name)?;

        let mut rule_names = vec![];

        for rule in rules {
            if rule.active {
                self.validate_rule(&node_path, rule)?;
                MatcherConfigValidator::check_unique_name(&mut rule_names, &rule.name)?;
            }
        }

        Ok(())
    }

    /// Checks that a rule:
    /// - has a valid name
    /// - has valid extracted variable names
    /// - has valid action IDs
    fn validate_rule(&self, parent: &NodePath, rule: &Rule) -> Result<(), MatcherError> {
        let rule_name = &rule.name;

        debug!("MatcherConfigValidator validate - Validating rule: [{}]", rule_name);
        let rule_node = NodePath::Parent { name: rule_name, parent, is_iterator: false };
        self.id.validate_rule_name(parent, rule_name)?;

        for var_name in rule.constraint.with.keys() {
            self.id.validate_extracted_var_name(&rule_node, var_name)?
        }

        for action in &rule.actions {
            self.id.validate_action_id(&rule_node, &action.id)?
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
            return Err(MatcherError::NotUniqueNameError { name: name_string });
        }
        rule_names.push(name_string);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::rule::{ConfigAction, Constraint, Extractor, ExtractorRegex, Operator};
    use crate::config::Defaultable;
    use serde_json::Map;
    use std::collections::HashMap;
    use tornado_common_api::Value;

    #[test]
    fn should_fail_if_wrong_ruleset_name() {
        // Arrange
        let rule = new_rule(
            "rule_name",
            Operator::Equals {
                first: Value::String("1".to_owned()),
                second: Value::String("1".to_owned()),
            },
        );

        // Act
        let result = MatcherConfigValidator::new().validate_ruleset("", &[rule], &NodePath::Root);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_validate_correct_rule() {
        // Arrange
        let rule = new_rule(
            "rule_name",
            Operator::Equals {
                first: Value::String("1".to_owned()),
                second: Value::String("1".to_owned()),
            },
        );

        // Act
        let result =
            MatcherConfigValidator::new().validate_ruleset("ruleset", &[rule], &NodePath::Root);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn should_validate_correct_rules() {
        // Arrange
        let rule_1 = new_rule(
            "rule_name",
            Operator::Equals {
                first: Value::String("1".to_owned()),
                second: Value::String("1".to_owned()),
            },
        );

        let rule_2 = new_rule(
            "rule_name_2",
            Operator::Equals {
                first: Value::String("1".to_owned()),
                second: Value::String("1".to_owned()),
            },
        );

        // Act
        let result = MatcherConfigValidator::new().validate_ruleset(
            "ruleset",
            &vec![rule_1, rule_2],
            &NodePath::Root,
        );

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn should_fail_validation_if_empty_name() {
        // Arrange
        let rule_1 = new_rule(
            "",
            Operator::Equals {
                first: Value::String("1".to_owned()),
                second: Value::String("1".to_owned()),
            },
        );

        // Act
        let result =
            MatcherConfigValidator::new().validate_ruleset("ruleset", &[rule_1], &NodePath::Root);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn build_should_fail_if_not_unique_name() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("1".to_owned()),
            second: Value::String("1".to_owned()),
        };
        let rule_1 = new_rule("rule_name", op.clone());
        let rule_2 = new_rule("rule_name", op);

        // Act
        let matcher = MatcherConfigValidator::new().validate_ruleset(
            "ruleset",
            &vec![rule_1, rule_2],
            &NodePath::Root,
        );

        // Assert
        assert!(matcher.is_err());

        match matcher.err().unwrap() {
            MatcherError::NotUniqueNameError { name } => assert_eq!("rule_name", name),
            _ => unreachable!(),
        }
    }

    #[test]
    fn build_should_fail_if_empty_spaces_in_rule_name() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("1".to_owned()),
            second: Value::String("1".to_owned()),
        };
        let rule_1 = new_rule("rule name", op);

        // Act
        let matcher =
            MatcherConfigValidator::new().validate_ruleset("ruleset", &[rule_1], &NodePath::Root);

        // Assert
        assert!(matcher.is_err());
    }

    #[test]
    fn build_should_fail_if_not_correct_name() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("1".to_owned()),
            second: Value::String("1".to_owned()),
        };
        let rule_1 = new_rule("rule.name", op);

        // Act
        let matcher =
            MatcherConfigValidator::new().validate_ruleset("ruleset", &[rule_1], &NodePath::Root);

        // Assert
        assert!(matcher.is_err());
    }

    #[test]
    fn build_should_fail_if_not_correct_extracted_var_name() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("1".to_owned()),
            second: Value::String("1".to_owned()),
        };
        let mut rule_1 = new_rule("rule_name", op);

        rule_1.constraint.with.insert(
            "var.with.dot".to_owned(),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
                modifiers_post: vec![],
            },
        );

        // Act
        let matcher =
            MatcherConfigValidator::new().validate_ruleset("ruleset", &[rule_1], &NodePath::Root);

        // Assert
        assert!(matcher.is_err());
    }

    #[test]
    fn build_should_fail_if_not_correct_action_id() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("1".to_owned()),
            second: Value::String("1".to_owned()),
        };
        let mut rule_1 = new_rule("rule_name", op);

        rule_1.actions.push(ConfigAction {
            id: "id.with.dot.and.question.mark?".to_owned(),
            payload: Map::new(),
        });

        // Act
        let matcher =
            MatcherConfigValidator::new().validate_ruleset("ruleset", &[rule_1], &NodePath::Root);

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
            &[],
            &NodePath::Root,
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
        let matcher = MatcherConfigValidator::new().validate_filter(
            "good_name",
            &filter,
            &[],
            &NodePath::Root,
        );

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
        let matcher = MatcherConfigValidator::new().validate_filter(
            "good_names",
            &filter,
            &[rules],
            &NodePath::Root,
        );

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
        let matcher = MatcherConfigValidator::new().validate_filter(
            "good_names",
            &filter,
            &[rules],
            &NodePath::Root,
        );

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

    #[test]
    fn test_node_path_rendering() {
        assert_eq!("root", format!("{}", NodePath::Root));
        assert_eq!(
            "root.master",
            format!(
                "{}",
                NodePath::Parent { name: "master", parent: &NodePath::Root, is_iterator: false }
            )
        );
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
