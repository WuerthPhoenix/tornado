use config::Rule;
use error::MatcherError;
use extractor::{MatcherExtractor, MatcherExtractorBuilder};
use operator;
use std::collections::HashMap;
use tornado_common_api::Event;

/// Matcher's internal Rule representation.
/// It contains the operators and executors built from the config::Rule
struct MatcherRule {
    name: String,
    priority: u16,
    do_continue: bool,
    operator: Box<operator::Operator>,
    extractor: MatcherExtractor,
}

/// The ProcessedEvent is the result of the matcher process.
/// It contains the original Event along with the result of the matching operation.
pub struct ProcessedEvent<'o> {
    pub event: Event,
    pub matched: HashMap<&'o str, HashMap<&'o str, String>>,
}

/// The Matcher contains the core logic of the Tornado Engine.
/// It matches the incoming Events with the defined Rules.
/// A Matcher instance is stateless and Thread safe; consequently, a single instance can serve the entire application.
pub struct Matcher {
    rules: Vec<MatcherRule>,
}

impl Matcher {
    /// Builds a new Matcher and configures it to operate with a set of Rules.
    pub fn new(rules: &[Rule]) -> Result<Matcher, MatcherError> {
        let operator_builder = operator::OperatorBuilder::new();
        let extractor_builder = MatcherExtractorBuilder::new();
        let mut processed_rules = vec![];

        let mut rule_names = vec![];
        let mut rules_by_priority = HashMap::new();

        for rule in rules {
            if rule.active {
                Matcher::check_unique_name(&mut rule_names, &rule.name)?;
                Matcher::check_unique_priority(&mut rules_by_priority, &rule)?;
                processed_rules.push(MatcherRule {
                    name: rule.name.to_owned(),
                    priority: rule.priority,
                    do_continue: rule.do_continue,
                    operator: operator_builder.build(&rule.constraint.where_operator)?,
                    extractor: extractor_builder.build(&rule.constraint.with)?,
                })
            }
        }

        // Sort rules by priority
        processed_rules.sort_by(|a, b| a.priority.cmp(&b.priority));

        Ok(Matcher {
            rules: processed_rules,
        })
    }

    fn check_unique_name(rule_names: &mut Vec<String>, name: &str) -> Result<(), MatcherError> {
        let name_string = name.to_owned();
        if rule_names.contains(&name_string) {
            return Err(MatcherError::NotUniqueRuleNameError { name: name_string });
        }
        rule_names.push(name_string);
        Ok(())
    }

    fn check_unique_priority(
        rules_by_priority: &mut HashMap<u16, String>,
        rule: &Rule,
    ) -> Result<(), MatcherError> {
        if rules_by_priority.contains_key(&rule.priority) {
            return Err(MatcherError::NotUniqueRulePriorityError {
                first_rule_name: rules_by_priority[&rule.priority].to_owned(),
                second_rule_name: rule.name.to_owned(),
                priority: rule.priority,
            });
        }
        rules_by_priority.insert(rule.priority, rule.name.to_owned());
        Ok(())
    }

    /// Processes an incoming Event against the set of Rules defined at Matcher's creation time.
    /// The result is a ProcessedEvent.
    pub fn process(&self, event: Event) -> ProcessedEvent {
        let mut processed_event = ProcessedEvent { event, matched: HashMap::new() };

        for rule in &self.rules {
            if rule.operator.evaluate(&processed_event.event) {
                match rule.extractor.extract_all(&processed_event.event) {
                    Ok(vars) => {
                        processed_event.matched.insert(rule.name.as_str(), vars);
                        if !rule.do_continue {
                            break;
                        }
                    }
                    // TODO: how to handle the error?
                    // Ignore Clippy for the moment.
                    Err(_) => {}
                }
            }
        }

        processed_event
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use config::{Constraint, Extractor, ExtractorRegex, Operator};
    use std::collections::HashMap;

    #[test]
    fn should_build_the_matcher() {
        // Arrange
        let rule = new_rule(
            "rule name",
            0,
            Operator::Equal {
                first: "1".to_owned(),
                second: "1".to_owned(),
            },
        );

        // Act
        let matcher = Matcher::new(&vec![rule]).unwrap();

        // Assert
        assert_eq!(1, matcher.rules.len());
        assert_eq!("rule name", matcher.rules[0].name);
    }

    #[test]
    fn build_should_fail_if_not_unique_name() {
        // Arrange
        let op = Operator::Equal {
            first: "1".to_owned(),
            second: "1".to_owned(),
        };
        let rule_1 = new_rule("rule_name", 0, op.clone());
        let rule_2 = new_rule("rule_name", 1, op.clone());

        // Act
        let matcher = Matcher::new(&vec![rule_1, rule_2]);

        // Assert
        assert!(matcher.is_err());

        match matcher.err().unwrap() {
            MatcherError::NotUniqueRuleNameError { name } => assert_eq!("rule_name", name),
            _ => assert!(false),
        }
    }

    #[test]
    fn build_should_fail_if_not_unique_priority() {
        // Arrange
        let op = Operator::Equal {
            first: "1".to_owned(),
            second: "1".to_owned(),
        };
        let rule_1 = new_rule("rule_1", 1, op.clone());
        let rule_2 = new_rule("rule_2", 1, op.clone());

        // Act
        let matcher = Matcher::new(&vec![rule_1, rule_2]);

        // Assert
        assert!(matcher.is_err());

        match matcher.err().unwrap() {
            MatcherError::NotUniqueRulePriorityError {
                first_rule_name,
                second_rule_name,
                priority,
            } => {
                assert_eq!("rule_1", first_rule_name);
                assert_eq!("rule_2", second_rule_name);
                assert_eq!(1, priority);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_sort_the_rules_based_on_priority() {
        // Arrange
        let op = Operator::Equal {
            first: "1".to_owned(),
            second: "1".to_owned(),
        };
        let rule_1 = new_rule("rule1", 10, op.clone());
        let rule_2 = new_rule("rule2", 1, op.clone());
        let rule_3 = new_rule("rule3", 1000, op.clone());
        let rule_4 = new_rule("rule4", 100, op.clone());

        // Act
        let matcher = Matcher::new(&vec![rule_1, rule_2, rule_3, rule_4]).unwrap();

        // Assert
        assert_eq!(4, matcher.rules.len());
        assert_eq!("rule2", matcher.rules[0].name);
        assert_eq!("rule1", matcher.rules[1].name);
        assert_eq!("rule4", matcher.rules[2].name);
        assert_eq!("rule3", matcher.rules[3].name);
    }

    #[test]
    fn should_ignore_non_active_rules() {
        // Arrange
        let op = Operator::Equal {
            first: "1".to_owned(),
            second: "1".to_owned(),
        };
        let mut rule_1 = new_rule("rule1", 0, op.clone());
        rule_1.active = false;

        let rule_2 = new_rule("rule2", 10, op.clone());

        let mut rule_3 = new_rule("rule3", 20, op.clone());
        rule_3.active = false;

        let rule_4 = new_rule("rule4", 30, op.clone());

        // Act
        let matcher = Matcher::new(&vec![rule_1, rule_2, rule_3, rule_4]).unwrap();

        // Assert
        assert_eq!(2, matcher.rules.len());
        assert_eq!("rule2", matcher.rules[0].name);
        assert_eq!("rule4", matcher.rules[1].name);
    }

    #[test]
    fn should_return_matching_rules() {
        // Arrange
        let rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "email".to_owned(),
            },
        );

        let rule_2 = new_rule(
            "rule2_sms",
            1,
            Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "sms".to_owned(),
            },
        );

        let rule_3 = new_rule(
            "rule3_email",
            2,
            Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "email".to_owned(),
            },
        );

        let matcher = Matcher::new(&vec![rule_1, rule_2, rule_3]).unwrap();

        // Act
        let result = matcher.process(Event {
            created_ts: 0,
            event_type: String::from("email"),
            payload: HashMap::new(),
        });

        // Assert
        assert_eq!(2, result.matched.len());
        assert!(result.matched.contains_key("rule1_email"));
        assert!(result.matched.contains_key("rule3_email"));
    }

    #[test]
    fn should_stop_execution_if_continue_is_false() {
        // Arrange
        let op = Operator::Equal {
            first: "${event.type}".to_owned(),
            second: "email".to_owned(),
        };

        let rule_1 = new_rule("rule1_email", 0, op.clone());

        let mut rule_2 = new_rule("rule2_email", 1, op.clone());
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", 2, op.clone());

        let matcher = Matcher::new(&vec![rule_1, rule_2, rule_3]).unwrap();

        // Act
        let result = matcher.process(Event {
            created_ts: 0,
            event_type: String::from("email"),
            payload: HashMap::new(),
        });

        // Assert
        assert_eq!(2, result.matched.len());
        assert!(result.matched.contains_key("rule1_email"));
        assert!(result.matched.contains_key("rule2_email"));
    }

    #[test]
    fn should_not_stop_execution_if_continue_is_false_in_a_non_matching_rule() {
        // Arrange
        let op = Operator::Equal {
            first: "${event.type}".to_owned(),
            second: "email".to_owned(),
        };

        let rule_1 = new_rule("rule1_email", 0, op.clone());

        let mut rule_2 = new_rule(
            "rule2_sms",
            1,
            Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "sms".to_owned(),
            },
        );
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", 2, op.clone());

        let matcher = Matcher::new(&vec![rule_1, rule_2, rule_3]).unwrap();

        // Act
        let result = matcher.process(Event {
            created_ts: 0,
            event_type: String::from("email"),
            payload: HashMap::new(),
        });

        // Assert
        assert_eq!(2, result.matched.len());
        assert!(result.matched.contains_key("rule1_email"));
        assert!(result.matched.contains_key("rule3_email"));
    }

    #[test]
    fn should_return_matching_rules_and_extracted_variables() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "email".to_owned(),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[ai]+"),
                    group_match_idx: 0,
                },
            },
        );

        let matcher = Matcher::new(&vec![rule_1]).unwrap();

        // Act
        let result = matcher.process(Event {
            created_ts: 0,
            event_type: String::from("email"),
            payload: HashMap::new(),
        });

        // Assert
        assert_eq!(1, result.matched.len());
        assert!(result.matched.contains_key("rule1_email"));

        let rule_1_processed = result.matched.get("rule1_email").unwrap();
        assert!(rule_1_processed.contains_key("extracted_temp"));
        assert_eq!("ai", rule_1_processed.get("extracted_temp").unwrap());
    }

    #[test]
    fn should_return_extracted_vars_grouped_by_rule() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "email".to_owned(),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[ai]+"),
                    group_match_idx: 0,
                },
            },
        );

        let mut rule_2 = new_rule(
            "rule2_email",
            1,
            Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "email".to_owned(),
            },
        );

        rule_2.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[em]+"),
                    group_match_idx: 0,
                },
            },
        );

        let matcher = Matcher::new(&vec![rule_1, rule_2]).unwrap();

        // Act
        let result = matcher.process(Event {
            created_ts: 0,
            event_type: String::from("email"),
            payload: HashMap::new(),
        });

        // Assert
        assert_eq!(2, result.matched.len());

        let rule_1_processed = result.matched.get("rule1_email").unwrap();
        assert!(rule_1_processed.contains_key("extracted_temp"));
        assert_eq!("ai", rule_1_processed.get("extracted_temp").unwrap());

        let rule_2_processed = result.matched.get("rule2_email").unwrap();
        assert!(rule_2_processed.contains_key("extracted_temp"));
        assert_eq!("em", rule_2_processed.get("extracted_temp").unwrap());
    }

    #[test]
    fn should_return_rule_only_if_matches_the_extracted_variables_too() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "email".to_owned(),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[z]+"),
                    group_match_idx: 0,
                },
            },
        );

        let mut rule_2 = new_rule(
            "rule2_email",
            1,
            Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "email".to_owned(),
            },
        );

        rule_2.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[ai]+"),
                    group_match_idx: 0,
                },
            },
        );

        let matcher = Matcher::new(&vec![rule_1, rule_2]).unwrap();

        // Act
        let result = matcher.process(Event {
            created_ts: 0,
            event_type: String::from("email"),
            payload: HashMap::new(),
        });

        // Assert
        assert_eq!(1, result.matched.len());
        assert!(result.matched.contains_key("rule2_email"));

        let rule_2_processed = result.matched.get("rule2_email").unwrap();
        assert!(rule_2_processed.contains_key("extracted_temp"));
        assert_eq!("ai", rule_2_processed.get("extracted_temp").unwrap());
    }

    fn new_rule(name: &str, priority: u16, operator: Operator) -> Rule {
        let constraint = Constraint {
            where_operator: operator,
            with: HashMap::new(),
        };

        Rule {
            name: name.to_owned(),
            priority,
            do_continue: true,
            active: true,
            actions: vec![],
            description: "".to_owned(),
            constraint,
        }
    }

}
