use config;
use config::Rule;
use error::MatcherError;
use operator;
use tornado_common_api::Event;

/// Matcher's internal Rule representation.
/// It contains the operators and executors built from the config::Rule
struct MatcherRule {
    name: String,
    priority: u16,
    do_continue: bool,
    operator: Box<operator::Operator>,
}

/// The ProcessedEvent is the result of the matcher process.
/// It contains the original Event along with the result of the matching operation.
pub struct ProcessedEvent {
    pub event: Event,
    pub matched: Vec<String>,
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
        let builder = operator::OperatorBuilder::new();
        let mut processed_rules = vec![];

        for rule in rules {
            if rule.active {
                processed_rules.push(MatcherRule {
                    name: rule.name.to_owned(),
                    priority: rule.priority,
                    do_continue: rule.do_continue,
                    operator: builder.build(&rule.constraint.where_operator)?,
                })
            }
        }

        // Sort rules by priority
        processed_rules.sort_by(|a, b| a.priority.cmp(&b.priority));

        Ok(Matcher {
            rules: processed_rules,
        })
    }

    /// Processes an incoming Event against the set of Rules defined at Matcher's creation time.
    /// The result is a ProcessedEvent.
    pub fn process(&self, event: Event) -> ProcessedEvent {
        let mut matched = vec![];

        for rule in &self.rules {
            let rule_name = &rule.name;
            if rule.operator.evaluate(&event) {
                matched.push(rule_name.to_owned());
                if !rule.do_continue {
                    break;
                }
            }
        }

        ProcessedEvent { event, matched }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn should_build_the_matcher() {
        // Arrange
        let rule = new_rule(
            "rule name",
            config::Operator::Equal {
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
    fn should_sort_the_rules_based_on_priority() {
        // Arrange
        let op = config::Operator::Equal {
            first: "1".to_owned(),
            second: "1".to_owned(),
        };
        let mut rule_1 = new_rule("rule1", op.clone());
        rule_1.priority = 10;

        let mut rule_2 = new_rule("rule2", op.clone());
        rule_2.priority = 1;

        let mut rule_3 = new_rule("rule3", op.clone());
        rule_3.priority = 1000;

        let mut rule_4 = new_rule("rule4", op.clone());
        rule_4.priority = 100;

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
        let op = config::Operator::Equal {
            first: "1".to_owned(),
            second: "1".to_owned(),
        };
        let mut rule_1 = new_rule("rule1", op.clone());
        rule_1.active = false;

        let rule_2 = new_rule("rule2", op.clone());

        let mut rule_3 = new_rule("rule3", op.clone());
        rule_3.active = false;

        let rule_4 = new_rule("rule4", op.clone());

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
            config::Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "email".to_owned(),
            },
        );

        let rule_2 = new_rule(
            "rule2_sms",
            config::Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "sms".to_owned(),
            },
        );

        let rule_3 = new_rule(
            "rule3_email",
            config::Operator::Equal {
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
        assert_eq!(String::from("rule1_email"), result.matched[0]);
        assert_eq!(String::from("rule3_email"), result.matched[1]);
    }

    #[test]
    fn should_stop_execution_if_continue_is_false() {
        // Arrange
        let op = config::Operator::Equal {
            first: "${event.type}".to_owned(),
            second: "email".to_owned(),
        };

        let rule_1 = new_rule("rule1_email", op.clone());

        let mut rule_2 = new_rule("rule2_email", op.clone());
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", op.clone());

        let matcher = Matcher::new(&vec![rule_1, rule_2, rule_3]).unwrap();

        // Act
        let result = matcher.process(Event {
            created_ts: 0,
            event_type: String::from("email"),
            payload: HashMap::new(),
        });

        // Assert
        assert_eq!(2, result.matched.len());
        assert_eq!(String::from("rule1_email"), result.matched[0]);
        assert_eq!(String::from("rule2_email"), result.matched[1]);
    }

    #[test]
    fn should_not_stop_execution_if_continue_is_false_in_a_non_matching_rule() {
        // Arrange
        let op = config::Operator::Equal {
            first: "${event.type}".to_owned(),
            second: "email".to_owned(),
        };

        let rule_1 = new_rule("rule1_email", op.clone());

        let mut rule_2 = new_rule(
            "rule2_sms",
            config::Operator::Equal {
                first: "${event.type}".to_owned(),
                second: "sms".to_owned(),
            },
        );
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", op.clone());

        let matcher = Matcher::new(&vec![rule_1, rule_2, rule_3]).unwrap();

        // Act
        let result = matcher.process(Event {
            created_ts: 0,
            event_type: String::from("email"),
            payload: HashMap::new(),
        });

        // Assert
        assert_eq!(2, result.matched.len());
        assert_eq!(String::from("rule1_email"), result.matched[0]);
        assert_eq!(String::from("rule3_email"), result.matched[1]);
    }

    fn new_rule(name: &str, operator: config::Operator) -> config::Rule {
        let constraint = config::Constraint {
            where_operator: operator,
            with: HashMap::new(),
        };

        config::Rule {
            name: name.to_owned(),
            priority: 0,
            do_continue: true,
            active: true,
            actions: vec![],
            description: "".to_owned(),
            constraint,
        }
    }

}
