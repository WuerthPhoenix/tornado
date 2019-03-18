pub mod action;
pub mod extractor;
pub mod operator;

use crate::config::MatcherConfig;
use crate::error::MatcherError;
use crate::matcher::extractor::{MatcherExtractor, MatcherExtractorBuilder};
use crate::model::{ProcessedEvent, ProcessedRule, ProcessedRuleStatus};
use crate::validator::MatcherConfigValidator;
use log::*;
use tornado_common_api::Event;

/// The Matcher's internal Rule representation, which contains the operators and executors built
///   from the config::rule::Rule.
pub struct MatcherRule {
    name: String,
    do_continue: bool,
    operator: Box<operator::Operator>,
    extractor: MatcherExtractor,
    actions: Vec<action::ActionResolver>,
}

/// The Matcher's internal Filter representation, which contains the operators and executors built
///   from the config::filter::Filter.
pub struct MatcherFilter {
    pub name: String,
    pub filter: Box<operator::Operator>,
}

pub enum ProcessingNode {
    DoNothing,
    Rules(Vec<MatcherRule>),
    Filter(MatcherFilter, Vec<ProcessingNode>),
}

/// The Matcher contains the core logic of the Tornado Engine.
/// It matches incoming Events against the defined Rules.
/// A Matcher instance is stateless and thread-safe; consequently, a single instance can serve the entire application.
pub struct Matcher {
    node: ProcessingNode,
}

impl Matcher {
    /// Builds a new Matcher and configures it to operate with a set of Rules.
    pub fn build(config: &MatcherConfig) -> Result<Matcher, MatcherError> {
        info!("Matcher build start");
        MatcherConfigValidator::new().validate(config)?;
        Matcher::build_processing_tree(config).map(|node| Matcher { node })
    }

    fn build_processing_tree(config: &MatcherConfig) -> Result<ProcessingNode, MatcherError> {
        match config {
            MatcherConfig::Rules { rules } => {
                info!("Start processing {} Matcher Config Rules", rules.len());


                let action_builder = action::ActionResolverBuilder::new();
                let operator_builder = operator::OperatorBuilder::new();
                let extractor_builder = MatcherExtractorBuilder::new();
                let mut processed_rules = vec![];

                for rule in rules.iter().filter(|rule| rule.active) {
                    info!("Matcher build - Processing rule: [{}]", &rule.name);
                    debug!("Matcher build - Processing rule definition:\n{:#?}", rule);

                    processed_rules.push(MatcherRule {
                        name: rule.name.to_owned(),
                        do_continue: rule.do_continue,
                        operator: operator_builder
                            .build_option(&rule.name, &rule.constraint.where_operator)?,
                        extractor: extractor_builder.build(&rule.name, &rule.constraint.with)?,
                        actions: action_builder.build(&rule.name, &rule.actions)?,
                    })
                }

                info!("Matcher Rules build completed");

                Ok(ProcessingNode::Rules(processed_rules))
            }
            MatcherConfig::Filter { filter, nodes } => {
                if !filter.active {
                    info!("Matcher Filter [{}] is not active. Ignore it.", filter.name);
                    return Ok(ProcessingNode::DoNothing);
                }

                info!("Start processing Matcher Filter [{}] Config", filter.name);
                let operator_builder = operator::OperatorBuilder::new();

                let matcher_filter = MatcherFilter {
                    name: filter.name.to_owned(),
                    filter: operator_builder.build_option(&filter.name, &filter.filter)?,
                };

                let matcher_nodes = nodes
                    .iter()
                    .map(|(_k, v)| Matcher::build_processing_tree(v))
                    .collect::<Result<Vec<_>, _>>()?;

                info!("Matcher Filter [{}] build completed", filter.name);
                Ok(ProcessingNode::Filter(matcher_filter, matcher_nodes))
            }
        }
    }

    /// Processes an incoming Event and compares it against the set of Rules defined at the Matcher's creation time.
    /// The result is a ProcessedEvent.
    pub fn process(&self, event: Event) -> ProcessedEvent {
        debug!("Matcher process - processing event: [{:#?}]", &event);
        let mut processed_event = ProcessedEvent::new(event);
        Matcher::process_node(&self.node, &mut processed_event);
        processed_event
    }

    fn process_node(node: &ProcessingNode, processed_event: &mut ProcessedEvent) {
        match node {
            ProcessingNode::Filter(filter, nodes) => {
                Matcher::process_filter(filter, nodes, processed_event)
            }
            ProcessingNode::Rules(rules) => Matcher::process_rules(rules, processed_event),
            ProcessingNode::DoNothing => {}
        };
    }

    fn process_filter(
        filter: &MatcherFilter,
        nodes: &[ProcessingNode],
        processed_event: &mut ProcessedEvent,
    ) {
        trace!("Matcher process - check matching of filter: [{}]", &filter.name);
        if filter.filter.evaluate(&processed_event) {
            trace!(
                    "Matcher process - event matches filter: [{}]. Passing the Event to the nested nodes.",
                    &filter.name
                );
            nodes.iter().for_each(|node| Matcher::process_node(node, processed_event));
        }
    }

    fn process_rules(rules: &[MatcherRule], mut processed_event: &mut ProcessedEvent) {
        for rule in rules {
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
    use crate::config::filter::Filter;
    use crate::config::rule::{Action, Constraint, Extractor, ExtractorRegex, Operator, Rule};
    use maplit::*;
    use std::collections::{BTreeMap, HashMap};
    use tornado_common_api::*;

    #[test]
    fn should_build_the_matcher_with_a_rule_set() {
        // Arrange
        let rule = new_rule(
            "rule_name",
            Operator::Equal { first: "1".to_owned(), second: "1".to_owned() },
        );

        // Act
        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule] }).unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Rules(rules) => {
                assert_eq!(1, rules.len());
                assert_eq!("rule_name", rules[0].name);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_build_the_matcher_with_a_filter() {
        // Arrange
        let filter = new_filter(
            "filter_name",
            Operator::Equal { first: "1".to_owned(), second: "1".to_owned() },
        );

        // Act
        let matcher =
            new_matcher(&MatcherConfig::Filter { filter, nodes: BTreeMap::new() }).unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Filter(filter, nodes) => {
                assert_eq!(0, nodes.len());
                assert_eq!("filter_name", filter.name);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_not_build_inner_nodes_if_filter_is_inactive() {
        // Arrange
        let mut filter = new_filter("filter_name", None);
        filter.active = false;

        // Act
        let matcher = new_matcher(&MatcherConfig::Filter {
            filter,
            nodes: btreemap!["node".to_owned() => MatcherConfig::Rules { rules: vec![new_rule("rule1", None)] }],
        })
        .unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::DoNothing => {
                assert!(true);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_build_the_matcher_with_a_filter_recursively() {
        // Arrange
        let filter = new_filter(
            "filter1",
            Operator::Equal { first: "1".to_owned(), second: "1".to_owned() },
        );

        let nodes = btreemap![
            "node1".to_owned() => MatcherConfig::Filter {
                filter: new_filter("filter2", None),
                nodes: btreemap!["node".to_owned() => MatcherConfig::Rules { rules: vec![new_rule("rule2", None)] }],
            },
            "node2".to_owned() => MatcherConfig::Rules { rules: vec![new_rule("rule1", None)] },
        ];

        let config = MatcherConfig::Filter { filter, nodes };

        // Act
        let matcher = new_matcher(&config).unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Filter(filter1, nodes1) => {
                assert_eq!(2, nodes1.len());
                assert_eq!("filter1", filter1.name);

                match &nodes1.get(0).unwrap() {
                    ProcessingNode::Filter(filter2, nodes2) => {
                        assert_eq!(1, nodes2.len());
                        assert_eq!("filter2", filter2.name);

                        match &nodes2.get(0).unwrap() {
                            ProcessingNode::Rules(rules2) => {
                                assert_eq!(1, rules2.len());
                                assert_eq!("rule2", rules2.get(0).unwrap().name);
                            }
                            _ => assert!(false),
                        }
                    }
                    _ => assert!(false),
                }

                match &nodes1.get(1).unwrap() {
                    ProcessingNode::Rules(rules1) => {
                        assert_eq!(1, rules1.len());
                        assert_eq!("rule1", rules1.get(0).unwrap().name);
                    }
                    _ => assert!(false),
                }
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn build_should_fail_if_not_unique_name() {
        // Arrange
        let op = Operator::Equal { first: "1".to_owned(), second: "1".to_owned() };
        let rule_1 = new_rule("rule_name", op.clone());
        let rule_2 = new_rule("rule_name", op.clone());

        // Act
        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule_1, rule_2] });

        // Assert
        assert!(matcher.is_err());

        match matcher.err().unwrap() {
            MatcherError::NotUniqueRuleNameError { name } => assert_eq!("rule_name", name),
            _ => assert!(false),
        }
    }

    #[test]
    fn should_sort_the_rules_based_on_input_order() {
        // Arrange
        let op = Operator::Equal { first: "1".to_owned(), second: "1".to_owned() };
        let rule_1 = new_rule("rule1", op.clone());
        let rule_2 = new_rule("rule2", op.clone());
        let rule_3 = new_rule("rule3", op.clone());
        let rule_4 = new_rule("rule4", op.clone());

        // Act
        let matcher =
            new_matcher(&MatcherConfig::Rules { rules: vec![rule_1, rule_2, rule_3, rule_4] })
                .unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Rules(rules) => {
                assert_eq!(4, rules.len());
                assert_eq!("rule1", rules[0].name);
                assert_eq!("rule2", rules[1].name);
                assert_eq!("rule3", rules[2].name);
                assert_eq!("rule4", rules[3].name);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_ignore_non_active_rules() {
        // Arrange
        let op = Operator::Equal { first: "1".to_owned(), second: "1".to_owned() };
        let mut rule_1 = new_rule("rule1", op.clone());
        rule_1.active = false;

        let rule_2 = new_rule("rule2", op.clone());

        let mut rule_3 = new_rule("rule3", op.clone());
        rule_3.active = false;

        let rule_4 = new_rule("rule4", op.clone());

        // Act
        let matcher =
            new_matcher(&MatcherConfig::Rules { rules: vec![rule_1, rule_2, rule_3, rule_4] })
                .unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Rules(rules) => {
                assert_eq!(2, rules.len());
                assert_eq!("rule2", rules[0].name);
                assert_eq!("rule4", rules[1].name);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_return_matching_rules() {
        // Arrange
        let rule_1 = new_rule(
            "rule1_email",
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        let rule_2 = new_rule(
            "rule2_sms",
            Operator::Equal { first: "${event.type}".to_owned(), second: "sms".to_owned() },
        );

        let rule_3 = new_rule(
            "rule3_email",
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        let matcher =
            new_matcher(&MatcherConfig::Rules { rules: vec![rule_1, rule_2, rule_3] }).unwrap();

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

        action
            .payload
            .insert("temp".to_owned(), Value::Text("${_variables.extracted_temp}".to_owned()));
        rule_1.actions.push(action);

        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule_1] }).unwrap();

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
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule_1] }).unwrap();

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
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.temp}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule_1] }).unwrap();

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

        action
            .payload
            .insert("temp".to_owned(), Value::Text("${_variables.extracted_temp}".to_owned()));
        action
            .payload
            .insert("missing".to_owned(), Value::Text("${_variables.missing}".to_owned()));
        rule_1.actions.push(action);

        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule_1] }).unwrap();

        let mut event_payload = HashMap::new();
        event_payload.insert(String::from("temp"), Value::Text(String::from("temp_value")));

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

        let rule_1 = new_rule("rule1_email", op.clone());

        let mut rule_2 = new_rule("rule2_email", op.clone());
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", op.clone());

        let matcher =
            new_matcher(&MatcherConfig::Rules { rules: vec![rule_1, rule_2, rule_3] }).unwrap();

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

        let rule_1 = new_rule("rule1_email", op.clone());

        let mut rule_2 = new_rule(
            "rule2_sms",
            Operator::Equal { first: "${event.type}".to_owned(), second: "sms".to_owned() },
        );
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", op.clone());

        let matcher =
            new_matcher(&MatcherConfig::Rules { rules: vec![rule_1, rule_2, rule_3] }).unwrap();

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
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule_1] }).unwrap();

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
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_2.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[em]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule_1, rule_2] }).unwrap();

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
            Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() },
        );

        rule_2.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule_1, rule_2] }).unwrap();

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

    #[test]
    fn should_match_rule_against_inner_array() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1",
            Operator::Equal {
                first: "${event.payload.array[0]}".to_owned(),
                second: "aaa".to_owned(),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.array[1]}"),
                regex: ExtractorRegex { regex: String::from(r"[z]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule_1] }).unwrap();

        let mut payload = Payload::new();
        payload.insert(
            "array".to_owned(),
            Value::Array(vec![Value::Text("aaa".to_owned()), Value::Text("zzz".to_owned())]),
        );

        // Act
        let result = matcher.process(Event::new_with_payload("email", payload));

        // Assert
        let rule_1_processed = result.rules.get("rule1").unwrap();
        assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
        assert_eq!(
            "zzz",
            result.extracted_vars.get("rule1.extracted_temp").unwrap().get_text().unwrap()
        );
    }

    #[test]
    fn should_match_rule_against_inner_map() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1",
            Operator::Equal {
                first: "${event.payload.map.key0}".to_owned(),
                second: "aaa".to_owned(),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.map.key1}"),
                regex: ExtractorRegex { regex: String::from(r"[z]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Rules { rules: vec![rule_1] }).unwrap();

        let mut payload = Payload::new();
        let mut inner = Payload::new();
        inner.insert("key0".to_owned(), Value::Text("aaa".to_owned()));
        inner.insert("key1".to_owned(), Value::Text("zzz".to_owned()));
        payload.insert("map".to_owned(), Value::Map(inner));

        // Act
        let result = matcher.process(Event::new_with_payload("email", payload));

        // Assert
        let rule_1_processed = result.rules.get("rule1").unwrap();
        assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
        assert_eq!(
            "zzz",
            result.extracted_vars.get("rule1.extracted_temp").unwrap().get_text().unwrap()
        );
    }

    #[test]
    fn should_process_rulesets_if_filter_has_no_operator() {
        // Arrange
        let op = Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() };

        let filter = new_filter("filter1", None);

        let nodes = btreemap![
            "node1".to_owned() => MatcherConfig::Rules { rules: vec![new_rule("rule_a1", op.clone())] },
            "node2".to_owned() => MatcherConfig::Rules { rules: vec![new_rule("rule_b1", op.clone())] },
        ];

        let config = MatcherConfig::Filter { filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(2, result.rules.len());
        assert!(result.rules.contains_key("rule_a1"));
        assert!(result.rules.contains_key("rule_b1"));
    }

    #[test]
    fn should_process_all_filter_rulesets() {
        // Arrange
        let op = Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() };

        let filter = new_filter("filter1", op.clone());

        let nodes = btreemap![
            "node1".to_owned() => MatcherConfig::Rules {
                rules: vec![new_rule("rule_a1", None), new_rule("rule_a2", op.clone())],
            },
            "node2".to_owned() => MatcherConfig::Rules {
                rules: vec![new_rule("rule_b1", None), new_rule("rule_b2", op.clone())],
            },
        ];

        let config = MatcherConfig::Filter { filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(4, result.rules.len());
        assert!(result.rules.contains_key("rule_a1"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_a1").unwrap().status);

        assert!(result.rules.contains_key("rule_a2"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_a2").unwrap().status);

        assert!(result.rules.contains_key("rule_b1"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_b1").unwrap().status);

        assert!(result.rules.contains_key("rule_b2"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_b2").unwrap().status);
    }

    #[test]
    fn should_process_filter_rulesets_recursively() {
        // Arrange
        let op = Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() };

        let filter = new_filter("filter1", None);

        let nodes = btreemap![
            "node0".to_owned() => MatcherConfig::Filter {
                filter: new_filter("filter2", op.clone()),
                nodes: btreemap![ "node".to_owned() => MatcherConfig::Rules { rules: vec![new_rule("rule2", None)] }],
            },
            "node1".to_owned() => MatcherConfig::Filter {
                filter: new_filter(
                    "filter3",
                    Operator::Equal {
                        first: "${event.type}".to_owned(),
                        second: "trap".to_owned(),
                    },
                ),
                nodes: btreemap![ "node".to_owned() => MatcherConfig::Rules { rules: vec![new_rule("rule3", None)] }],
            },
            "node2".to_owned() => MatcherConfig::Rules {
                rules: vec![new_rule("rule_a1", None), new_rule("rule_a2", op.clone())],
            },
            "node3".to_owned() => MatcherConfig::Rules {
                rules: vec![new_rule("rule_b1", None), new_rule("rule_b2", op.clone())],
            },
        ];

        let config = MatcherConfig::Filter { filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(5, result.rules.len());
        assert!(result.rules.contains_key("rule_a1"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_a1").unwrap().status);

        assert!(result.rules.contains_key("rule_a2"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_a2").unwrap().status);

        assert!(result.rules.contains_key("rule_b1"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_b1").unwrap().status);

        assert!(result.rules.contains_key("rule_b2"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_b2").unwrap().status);

        assert!(result.rules.contains_key("rule2"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule2").unwrap().status);
    }

    #[test]
    fn should_process_no_rulesets_if_filter_is_inactive() {
        // Arrange
        let op = Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() };

        let mut filter = new_filter("filter1", None);
        filter.active = false;

        let nodes = btreemap![
            "node0".to_owned() => MatcherConfig::Rules { rules: vec![new_rule("rule_a1", op.clone())] },
            "node1".to_owned() => MatcherConfig::Rules { rules: vec![new_rule("rule_b1", op.clone())] },
        ];

        let config = MatcherConfig::Filter { filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(0, result.rules.len());
    }

    #[test]
    fn should_process_no_rulesets_if_filter_does_not_match() {
        // Arrange
        let op = Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() };

        let filter = new_filter(
            "filter1",
            Operator::Equal { first: "${event.type}".to_owned(), second: "trapd".to_owned() },
        );

        let nodes = btreemap![
            "node0".to_owned() => MatcherConfig::Rules {
                rules: vec![new_rule("rule_a1", op.clone()), new_rule("rule_a2", op.clone())],
            },
            "node1".to_owned() => MatcherConfig::Rules {
                rules: vec![new_rule("rule_b1", op.clone()), new_rule("rule_b2", op.clone())],
            },
        ];

        let config = MatcherConfig::Filter { filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(0, result.rules.len());
    }

    #[test]
    fn should_process_rulesets_independently() {
        // Arrange
        let op = Operator::Equal { first: "${event.type}".to_owned(), second: "email".to_owned() };

        let filter = new_filter("filter1", op.clone());

        let mut rule_a1 = new_rule("rule_a1", None);
        rule_a1.do_continue = false;

        let mut rule_b1 = new_rule("rule_b1", None);
        rule_b1.do_continue = false;

        let mut rule_c1 = new_rule("rule_c1", None);
        rule_c1.do_continue = false;

        let nodes = btreemap![
            "node0".to_owned() => MatcherConfig::Rules { rules: vec![rule_a1, new_rule("rule_a2", op.clone())] },
            "node1".to_owned() => MatcherConfig::Rules { rules: vec![rule_b1, new_rule("rule_b2", op.clone())] },
            "node2".to_owned() => MatcherConfig::Rules { rules: vec![rule_c1, new_rule("rule_c2", op.clone())] },
        ];

        let config = MatcherConfig::Filter { filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        assert_eq!(3, result.rules.len());
        assert!(result.rules.contains_key("rule_a1"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_a1").unwrap().status);

        assert!(result.rules.contains_key("rule_b1"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_b1").unwrap().status);

        assert!(result.rules.contains_key("rule_c1"));
        assert_eq!(ProcessedRuleStatus::Matched, result.rules.get("rule_c1").unwrap().status);
    }

    fn new_matcher(config: &MatcherConfig) -> Result<Matcher, MatcherError> {
        //crate::test_root::start_context();
        Matcher::build(config)
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

    fn new_filter<O: Into<Option<Operator>>>(name: &str, filter: O) -> Filter {
        Filter {
            name: name.to_owned(),
            active: true,
            description: "".to_owned(),
            filter: filter.into(),
        }
    }

}
