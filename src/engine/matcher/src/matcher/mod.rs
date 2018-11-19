pub mod action;
pub mod extractor;
pub mod operator;

use config::Rule;
use error::MatcherError;
use matcher::extractor::{MatcherExtractor, MatcherExtractorBuilder};
use model::{ProcessedEvent, ProcessedRule, ProcessedRuleStatus};
use tornado_common_api::Event;
use validator::RuleValidator;

/// Matcher's internal Rule representation.
/// It contains the operators and executors built from the config::Rule
struct MatcherRule {
    name: String,
    priority: u16,
    do_continue: bool,
    operator: Box<operator::Operator>,
    extractor: MatcherExtractor,
    actions: Vec<action::ActionResolver>,
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
        info!("Matcher build start");

        RuleValidator::new().validate_all(rules)?;

        let action_builder = action::ActionResolverBuilder::new();
        let operator_builder = operator::OperatorBuilder::new();
        let extractor_builder = MatcherExtractorBuilder::new();
        let mut processed_rules = vec![];

        for rule in rules {
            if rule.active {
                info!("Matcher build - Processing rule: [{}]", &rule.name);
                debug!("Matcher build - Processing rule definition:\n{:#?}", rule);

                processed_rules.push(MatcherRule {
                    name: rule.name.to_owned(),
                    priority: rule.priority,
                    do_continue: rule.do_continue,
                    operator: operator_builder
                        .build(&rule.name, &rule.constraint.where_operator)?,
                    extractor: extractor_builder.build(&rule.name, &rule.constraint.with)?,
                    actions: action_builder.build(&rule.name, &rule.actions)?,
                })
            }
        }

        // Sort rules by priority
        processed_rules.sort_by(|a, b| a.priority.cmp(&b.priority));

        info!("Matcher build completed");

        Ok(Matcher { rules: processed_rules })
    }

    /// Processes an incoming Event against the set of Rules defined at Matcher's creation time.
    /// The result is a ProcessedEvent.
    pub fn process(&self, event: Event) -> ProcessedEvent {
        debug!("Matcher process - processing event: [{:#?}]", &event);

        let mut processed_event = ProcessedEvent::new(event);

        for rule in &self.rules {
            trace!("Matcher process - check matching of rule: [{}]", &rule.name);

            let mut processed_rule = ProcessedRule {
                rule_name: rule.name.clone(),
                status: ProcessedRuleStatus::NotMatched,
                actions: vec![],
                message: None,
            };

            if rule.operator.evaluate(&processed_event) {
                trace!(
                    "Matcher process - event matches rule: [{}]. Checking extracted variables.",
                    &rule.name
                );

                match rule.extractor.process_all(&mut processed_event) {
                    Ok(_) => {
                        trace!("Matcher process - event matches rule: [{}] and its extracted variables.", &rule.name);

                        match Matcher::process_actions(
                            &processed_event,
                            &mut processed_rule,
                            &rule.actions,
                        ) {
                            Ok(_) => {
                                processed_rule.status = ProcessedRuleStatus::Matched;
                                if !rule.do_continue {
                                    processed_event.rules.insert(rule.name.clone(), processed_rule);
                                    break;
                                }
                            }
                            Err(e) => {
                                let message = format!("Matcher process - The event matches the rule [{}] and all variables are extracted correctly; however, some actions cannot be resolved: [{}]", &rule.name, e.to_string());
                                debug!("{}", &message);
                                processed_rule.status = ProcessedRuleStatus::PartiallyMatched;
                                processed_rule.message = Some(message);
                            }
                        }
                    }
                    Err(e) => {
                        let message = format!("Matcher process - The event matches the rule [{}] but some variables cannot be extracted: [{}]", &rule.name, e.to_string());
                        debug!("{}", &message);
                        processed_rule.status = ProcessedRuleStatus::PartiallyMatched;
                        processed_rule.message = Some(message);
                    }
                }
            }

            processed_event.rules.insert(rule.name.clone(), processed_rule);
        }
        debug!("Matcher process - event processing result: [{:#?}]", &processed_event);
        processed_event
    }

    fn process_actions(
        processed_event: &ProcessedEvent,
        processed_rule: &mut ProcessedRule,
        actions: &[action::ActionResolver],
    ) -> Result<(), MatcherError> {
        for action in actions {
            processed_rule.actions.push(action.execute(processed_event)?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use config::{Action, Constraint, Extractor, ExtractorRegex, Operator};
    use std::collections::HashMap;
    use test_root;

    #[test]
    fn should_build_the_matcher() {
        // Arrange
        let rule = new_rule(
            "rule_name",
            0,
            Operator::Equal { first: "1".to_owned(), second: "1".to_owned() },
        );

        // Act
        let matcher = new_matcher(&vec![rule]).unwrap();

        // Assert
        assert_eq!(1, matcher.rules.len());
        assert_eq!("rule_name", matcher.rules[0].name);
    }

    #[test]
    fn build_should_fail_if_not_unique_name() {
        // Arrange
        let op = Operator::Equal { first: "1".to_owned(), second: "1".to_owned() };
        let rule_1 = new_rule("rule_name", 0, op.clone());
        let rule_2 = new_rule("rule_name", 1, op.clone());

        // Act
        let matcher = new_matcher(&vec![rule_1, rule_2]);

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
        let op = Operator::Equal { first: "1".to_owned(), second: "1".to_owned() };
        let rule_1 = new_rule("rule_1", 1, op.clone());
        let rule_2 = new_rule("rule_2", 1, op.clone());

        // Act
        let matcher = new_matcher(&vec![rule_1, rule_2]);

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
        let op = Operator::Equal { first: "1".to_owned(), second: "1".to_owned() };
        let rule_1 = new_rule("rule1", 10, op.clone());
        let rule_2 = new_rule("rule2", 1, op.clone());
        let rule_3 = new_rule("rule3", 1000, op.clone());
        let rule_4 = new_rule("rule4", 100, op.clone());

        // Act
        let matcher = new_matcher(&vec![rule_1, rule_2, rule_3, rule_4]).unwrap();

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
        let op = Operator::Equal { first: "1".to_owned(), second: "1".to_owned() };
        let mut rule_1 = new_rule("rule1", 0, op.clone());
        rule_1.active = false;

        let rule_2 = new_rule("rule2", 10, op.clone());

        let mut rule_3 = new_rule("rule3", 20, op.clone());
        rule_3.active = false;

        let rule_4 = new_rule("rule4", 30, op.clone());

        // Act
        let matcher = new_matcher(&vec![rule_1, rule_2, rule_3, rule_4]).unwrap();

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
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        let rule_2 = new_rule(
            "rule2_sms",
            1,
            Operator::Equal { first: "${event.type}".to_owned(), second: "sms".to_owned() },
        );

        let rule_3 = new_rule(
            "rule3_email",
            2,
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        let matcher = new_matcher(&vec![rule_1, rule_2, rule_3]).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(3, result.rules.len());
        assert!(result.rules.contains_key("rule1_email"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule1_email").unwrap().status);
        assert!(result.rules.contains_key("rule2_sms"));
        assert_eq!(ProcessedRuleStatus::NotMatched, result.rules.get("rule2_sms").unwrap().status);
        assert!(result.rules.contains_key("rule3_email"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule3_email").unwrap().status);
    }

    #[test]
    fn should_return_status_matched() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let mut action = Action { id: String::from("action_id"), payload: HashMap::new() };

        action.payload.insert("temp".to_owned(), "${_variables.extracted_temp}".to_owned());
        rule_1.actions.push(action);

        let matcher = new_matcher(&vec![rule_1]).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(1, result.rules.len());
        assert!(result.rules.contains_key("rule1_email"));

        let processed_rule = result.rules.get("rule1_email").unwrap();
        assert_eq!(ProcessedRuleStatus::Matched, processed_rule.status);
        assert_eq!(1, result.extracted_vars.len());
        assert_eq!("ai", result.extracted_vars.get("rule1_email.extracted_temp").unwrap());
        assert_eq!(1, processed_rule.actions.len());
        assert_eq!("ai", processed_rule.actions[0].payload.get("temp").unwrap());
        assert!(processed_rule.message.is_none())
    }

    #[test]
    fn should_return_status_not_matched_if_where_returns_false() {
        // Arrange
        let rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        let matcher = new_matcher(&vec![rule_1]).unwrap();

        // Act
        let result = matcher.process(Event::new("sms"));

        // Assert
        assert_eq!(1, result.rules.len());
        assert!(result.rules.contains_key("rule1_email"));

        let processed_rule = result.rules.get("rule1_email").unwrap();
        assert_eq!(ProcessedRuleStatus::NotMatched, processed_rule.status);
    }

    #[test]
    fn should_return_status_partially_matched_if_extracted_var_is_missing() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.temp}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&vec![rule_1]).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(1, result.rules.len());
        assert!(result.rules.contains_key("rule1_email"));

        let processed_rule = result.rules.get("rule1_email").unwrap();
        assert_eq!(ProcessedRuleStatus::PartiallyMatched, processed_rule.status);

        info!("Message: {:?}", processed_rule.message);
        assert!(processed_rule.message.clone().unwrap().contains("extracted_temp"))
    }

    #[test]
    fn should_return_status_partially_matched_if_action_payload_cannot_be_resolved() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.temp}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let mut action = Action { id: String::from("action_id"), payload: HashMap::new() };

        action.payload.insert("temp".to_owned(), "${_variables.extracted_temp}".to_owned());
        action.payload.insert("missing".to_owned(), "${_variables.missing}".to_owned());
        rule_1.actions.push(action);

        let matcher = new_matcher(&vec![rule_1]).unwrap();

        let mut event_payload = HashMap::new();
        event_payload.insert(String::from("temp"), String::from("temp_value"));

        // Act
        let result = matcher.process(Event::new_with_payload("email", event_payload));

        // Assert
        assert_eq!(1, result.rules.len());
        assert!(result.rules.contains_key("rule1_email"));

        let processed_rule = result.rules.get("rule1_email").unwrap();
        assert_eq!(ProcessedRuleStatus::PartiallyMatched, processed_rule.status);

        info!("Message: {:?}", processed_rule.message);
        assert!(processed_rule.message.clone().unwrap().contains("rule1_email.missing"))
    }

    #[test]
    fn should_stop_execution_if_continue_is_false() {
        // Arrange
        let op = Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() };

        let rule_1 = new_rule("rule1_email", 0, op.clone());

        let mut rule_2 = new_rule("rule2_email", 1, op.clone());
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", 2, op.clone());

        let matcher = new_matcher(&vec![rule_1, rule_2, rule_3]).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(2, result.rules.len());
        assert!(result.rules.contains_key("rule1_email"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule1_email").unwrap().status);
        assert!(result.rules.contains_key("rule2_email"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule2_email").unwrap().status);
    }
    #[test]
    fn should_not_stop_execution_if_continue_is_false_in_a_non_matching_rule() {
        // Arrange
        let op = Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() };

        let rule_1 = new_rule("rule1_email", 0, op.clone());

        let mut rule_2 = new_rule(
            "rule2_sms",
            1,
            Operator::Equal { first: "${event.type}".to_owned(), second: "sms".to_owned() },
        );
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", 2, op.clone());

        let matcher = new_matcher(&vec![rule_1, rule_2, rule_3]).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(3, result.rules.len());
        assert!(result.rules.contains_key("rule1_email"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule1_email").unwrap().status);
        assert!(result.rules.contains_key("rule2_sms"));
        assert_eq!(ProcessedRuleStatus::NotMatched, result.rules.get("rule2_sms").unwrap().status);
        assert!(result.rules.contains_key("rule3_email"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule3_email").unwrap().status);
    }

    #[test]
    fn should_return_matching_rules_and_extracted_variables() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&vec![rule_1]).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(1, result.rules.len());
        assert!(result.rules.contains_key("rule1_email"));

        let rule_1_processed = result.rules.get("rule1_email").unwrap();
        assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
        assert!(result.extracted_vars.contains_key("rule1_email.extracted_temp"));
        assert_eq!("ai", result.extracted_vars.get("rule1_email.extracted_temp").unwrap());
    }
    #[test]
    fn should_return_extracted_vars_grouped_by_rule() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let mut rule_2 = new_rule(
            "rule2_email",
            1,
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_2.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[em]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&vec![rule_1, rule_2]).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(2, result.rules.len());

        let rule_1_processed = result.rules.get("rule1_email").unwrap();
        assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
        assert!(result.extracted_vars.contains_key("rule1_email.extracted_temp"));
        assert_eq!("ai", result.extracted_vars.get("rule1_email.extracted_temp").unwrap());

        let rule_2_processed = result.rules.get("rule2_email").unwrap();
        assert_eq!(ProcessedRuleStatus::Matched, rule_2_processed.status);
        assert!(result.extracted_vars.contains_key("rule2_email.extracted_temp"));
        assert_eq!("em", result.extracted_vars.get("rule2_email.extracted_temp").unwrap());
    }

    #[test]
    fn should_return_rule_only_if_matches_the_extracted_variables_too() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            0,
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[z]+"), group_match_idx: 0 },
            },
        );

        let mut rule_2 = new_rule(
            "rule2_email",
            1,
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_2.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&vec![rule_1, rule_2]).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(2, result.rules.len());

        let rule_1_processed = result.rules.get("rule1_email").unwrap();
        assert_eq!(ProcessedRuleStatus::PartiallyMatched, rule_1_processed.status);
        assert!(!result.extracted_vars.contains_key("rule1_email.extracted_temp"));

        let rule_2_processed = result.rules.get("rule2_email").unwrap();
        assert_eq!(ProcessedRuleStatus::Matched, rule_2_processed.status);
        assert!(result.extracted_vars.contains_key("rule2_email.extracted_temp"));
        assert_eq!("ai", result.extracted_vars.get("rule2_email.extracted_temp").unwrap());
    }

    fn new_matcher(rules: &[Rule]) -> Result<Matcher, MatcherError> {
        test_root::start_context();
        Matcher::new(rules)
    }

    fn new_rule(name: &str, priority: u16, operator: Operator) -> Rule {
        let constraint = Constraint { where_operator: operator, with: HashMap::new() };

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
