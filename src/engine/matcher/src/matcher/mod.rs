pub mod action;
pub mod extractor;
pub mod operator;

use crate::config::MatcherConfig;
use crate::error::MatcherError;
use crate::matcher::extractor::{MatcherExtractor, MatcherExtractorBuilder};
use crate::model::{
    InternalEvent, ProcessedEvent, ProcessedFilter, ProcessedFilterStatus, ProcessedNode,
    ProcessedRule, ProcessedRuleStatus, ProcessedRules,
};
use crate::validator::MatcherConfigValidator;
use log::*;
use std::collections::HashMap;
use tornado_common_api::{Event, Value};

/// The Matcher's internal Rule representation, which contains the operators and executors built
///   from the config::rule::Rule.
pub struct MatcherRule {
    name: String,
    do_continue: bool,
    operator: Box<dyn operator::Operator>,
    extractor: MatcherExtractor,
    actions: Vec<action::ActionResolver>,
}

/// The Matcher's internal Filter representation, which contains the operators and executors built
///   from the config::filter::Filter.
pub struct MatcherFilter {
    pub active: bool,
    pub filter: Box<dyn operator::Operator>,
}

pub enum ProcessingNode {
    Filter { name: String, filter: MatcherFilter, nodes: Vec<ProcessingNode> },
    Ruleset { name: String, rules: Vec<MatcherRule> },
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
            MatcherConfig::Ruleset { name, rules } => {
                info!("Start processing {} Matcher Config Rules", rules.len());

                let action_builder = action::ActionResolverBuilder::new();
                let operator_builder = operator::OperatorBuilder::new();
                let extractor_builder = MatcherExtractorBuilder::new();
                let mut processed_rules = vec![];

                for rule in rules.iter().filter(|rule| rule.active) {
                    debug!("Matcher build - Processing rule: [{}]", &rule.name);
                    trace!("Matcher build - Processing rule definition:\n{:?}", rule);

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

                Ok(ProcessingNode::Ruleset { name: name.to_owned(), rules: processed_rules })
            }
            MatcherConfig::Filter { name, filter, nodes } => {
                debug!("Start processing Matcher Filter [{}] Config", name);
                let operator_builder = operator::OperatorBuilder::new();

                let matcher_filter = MatcherFilter {
                    active: filter.active,
                    filter: operator_builder.build_option(name, &filter.filter)?,
                };

                let mut matcher_nodes = vec![];
                if matcher_filter.active {
                    for node in nodes {
                        matcher_nodes.push(Matcher::build_processing_tree(node)?);
                    }
                };

                debug!("Matcher Filter [{}] build completed", name);
                Ok(ProcessingNode::Filter {
                    name: name.to_owned(),
                    filter: matcher_filter,
                    nodes: matcher_nodes,
                })
            }
        }
    }

    /// Processes an incoming Event and compares it against the set of Rules defined at the Matcher's creation time.
    /// The result is a ProcessedEvent.
    pub fn process(&self, event: Event) -> ProcessedEvent {
        trace!("Matcher process - processing event: [{:?}]", &event);
        let internal_event: InternalEvent = event.into();
        let result = Matcher::process_node(&self.node, &internal_event);
        ProcessedEvent { event: internal_event, result }
    }

    fn process_node(node: &ProcessingNode, internal_event: &InternalEvent) -> ProcessedNode {
        match node {
            ProcessingNode::Filter { name, filter, nodes } => {
                Matcher::process_filter(name, filter, nodes, internal_event)
            }
            ProcessingNode::Ruleset { name, rules } => {
                Matcher::process_rules(name, rules, internal_event)
            }
        }
    }

    fn process_filter(
        filter_name: &str,
        filter: &MatcherFilter,
        nodes: &[ProcessingNode],
        internal_event: &InternalEvent,
    ) -> ProcessedNode {
        trace!("Matcher process - check matching of filter: [{}]", filter_name);

        let mut result_nodes = vec![];

        let filter_status = if filter.active {
            if filter.filter.evaluate(&internal_event, None) {
                trace!(
                        "Matcher process - event matches filter: [{}]. Passing the Event to the nested nodes.",
                        filter_name
                    );
                nodes.iter().for_each(|node| {
                    let processed_node = Matcher::process_node(node, internal_event);
                    result_nodes.push(processed_node);
                });
                ProcessedFilterStatus::Matched
            } else {
                ProcessedFilterStatus::NotMatched
            }
        } else {
            ProcessedFilterStatus::Inactive
        };

        ProcessedNode::Filter {
            name: filter_name.to_owned(),
            filter: ProcessedFilter { status: filter_status },
            nodes: result_nodes,
        }
    }

    fn process_rules(
        ruleset_name: &str,
        rules: &[MatcherRule],
        internal_event: &InternalEvent,
    ) -> ProcessedNode {
        trace!("Matcher process - check matching of ruleset: [{}]", ruleset_name);
        let mut extracted_vars = HashMap::new();
        let mut processed_rules = vec![];

        for rule in rules {
            trace!("Matcher process - check matching of rule: [{}]", &rule.name);

            let mut processed_rule = ProcessedRule {
                name: rule.name.clone(),
                status: ProcessedRuleStatus::NotMatched,
                actions: vec![],
                message: None,
            };

            if rule.operator.evaluate(internal_event, Some(&extracted_vars)) {
                trace!(
                    "Matcher process - event matches rule: [{}]. Checking extracted variables.",
                    &rule.name
                );

                match rule.extractor.process_all(&internal_event, &mut extracted_vars) {
                    Ok(_) => {
                        trace!("Matcher process - event matches rule: [{}] and its extracted variables.", &rule.name);

                        match Matcher::process_actions(
                            internal_event,
                            Some(&extracted_vars),
                            &mut processed_rule,
                            &rule.actions,
                        ) {
                            Ok(_) => {
                                processed_rule.status = ProcessedRuleStatus::Matched;
                                if !rule.do_continue {
                                    processed_rules.push(processed_rule);
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

            processed_rules.push(processed_rule);
        }

        let result = ProcessedNode::Ruleset {
            name: ruleset_name.to_owned(),
            rules: ProcessedRules { rules: processed_rules, extracted_vars },
        };
        trace!("Matcher process - event processing rules result: [{:?}]", &result);
        result
    }

    fn process_actions(
        processed_event: &InternalEvent,
        extracted_vars: Option<&HashMap<String, Value>>,
        processed_rule: &mut ProcessedRule,
        actions: &[action::ActionResolver],
    ) -> Result<(), MatcherError> {
        for action in actions {
            processed_rule.actions.push(action.execute(processed_event, extracted_vars)?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::filter::Filter;
    use crate::config::rule::{Action, Constraint, Extractor, ExtractorRegex, Operator, Rule};
    use std::collections::HashMap;
    use tornado_common_api::*;

    #[test]
    fn should_build_the_matcher_with_a_rule_set() {
        // Arrange
        let rule = new_rule(
            "rule_name",
            Operator::Equal {
                first: Value::Text("1".to_owned()),
                second: Value::Text("1".to_owned()),
            },
        );

        // Act
        let matcher =
            new_matcher(&MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![rule] })
                .unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Ruleset { name, rules } => {
                assert_eq!(name, "ruleset");
                assert_eq!(1, rules.len());
                assert_eq!("rule_name", rules[0].name);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_build_the_matcher_with_a_filter() {
        // Arrange
        let filter = new_filter(Operator::Equal {
            first: Value::Text("1".to_owned()),
            second: Value::Text("1".to_owned()),
        });

        // Act
        let matcher = new_matcher(&MatcherConfig::Filter {
            name: "filter".to_owned(),
            filter,
            nodes: vec![],
        })
        .unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Filter { name, filter: _filter, nodes } => {
                assert_eq!(0, nodes.len());
                assert_eq!("filter", name);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_not_build_inner_nodes_if_filter_is_inactive() {
        // Arrange
        let mut filter = new_filter(None);
        filter.active = false;

        // Act
        let matcher = new_matcher(&MatcherConfig::Filter {
            name: "filter".to_owned(),
            filter,
            nodes: vec![MatcherConfig::Ruleset {
                name: "ruleset".to_owned(),
                rules: vec![new_rule("rule1", None)],
            }],
        })
        .unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Filter { name, filter: _filter, nodes } => {
                assert_eq!(0, nodes.len());
                assert_eq!("filter", name)
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_build_the_matcher_with_a_filter_recursively() {
        // Arrange
        let filter = new_filter(Operator::Equal {
            first: Value::Text("1".to_owned()),
            second: Value::Text("1".to_owned()),
        });

        let nodes = vec![
            MatcherConfig::Filter {
                name: "node1".to_owned(),
                filter: new_filter(None),
                nodes: vec![MatcherConfig::Ruleset {
                    name: "node3".to_owned(),
                    rules: vec![new_rule("rule2", None)],
                }],
            },
            MatcherConfig::Ruleset {
                name: "node2".to_owned(),
                rules: vec![new_rule("rule1", None)],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        // Act
        let matcher = new_matcher(&config).unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Filter { name, filter: _filter1, nodes: nodes1 } => {
                assert_eq!(2, nodes1.len());
                assert_eq!("filter", name);

                match &nodes1.get(0).unwrap() {
                    ProcessingNode::Filter { name, filter: _filter2, nodes: nodes2 } => {
                        assert_eq!(1, nodes2.len());
                        assert_eq!("node1", name);

                        match &nodes2.get(0).unwrap() {
                            ProcessingNode::Ruleset { rules: rules2, .. } => {
                                assert_eq!(1, rules2.len());
                                assert_eq!("rule2", rules2.get(0).unwrap().name);
                            }
                            _ => assert!(false),
                        }
                    }
                    _ => assert!(false),
                }

                match &nodes1.get(1).unwrap() {
                    ProcessingNode::Ruleset { name, rules: rules1 } => {
                        assert_eq!("node2", name);
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
        let op = Operator::Equal {
            first: Value::Text("1".to_owned()),
            second: Value::Text("1".to_owned()),
        };
        let rule_1 = new_rule("rule_name", op.clone());
        let rule_2 = new_rule("rule_name", op.clone());

        // Act
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2],
        });

        // Assert
        assert!(matcher.is_err());

        match matcher.err().unwrap() {
            MatcherError::NotUniqueRuleNameError { name } => assert_eq!("rule_name", name),
            _ => assert!(false),
        }
    }

    #[test]
    fn build_should_fail_if_filter_has_wrong_name() {
        // Arrange
        let filter = new_filter(None);
        let nodes = vec![];
        let config = MatcherConfig::Filter { name: "filter?!!".to_owned(), filter, nodes };

        let matcher = new_matcher(&config);

        // Act
        assert!(matcher.is_err());

        match matcher.err().unwrap() {
            MatcherError::NotValidIdOrNameError { message } => {
                assert!(message.contains("filter?!!"));
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_sort_the_rules_based_on_input_order() {
        // Arrange
        let op = Operator::Equal {
            first: Value::Text("1".to_owned()),
            second: Value::Text("1".to_owned()),
        };
        let rule_1 = new_rule("rule1", op.clone());
        let rule_2 = new_rule("rule2", op.clone());
        let rule_3 = new_rule("rule3", op.clone());
        let rule_4 = new_rule("rule4", op.clone());

        // Act
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2, rule_3, rule_4],
        })
        .unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
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
        let op = Operator::Equal {
            first: Value::Text("1".to_owned()),
            second: Value::Text("1".to_owned()),
        };
        let mut rule_1 = new_rule("rule1", op.clone());
        rule_1.active = false;

        let rule_2 = new_rule("rule2", op.clone());

        let mut rule_3 = new_rule("rule3", op.clone());
        rule_3.active = false;

        let rule_4 = new_rule("rule4", op.clone());

        // Act
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2, rule_3, rule_4],
        })
        .unwrap();

        // Assert
        match &matcher.node {
            ProcessingNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
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
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
        );

        let rule_2 = new_rule(
            "rule2_sms",
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("sms".to_owned()),
            },
        );

        let rule_3 = new_rule(
            "rule3_email",
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2, rule_3],
        })
        .unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(3, rules.rules.len());

                assert_eq!(rules.rules.get(0).unwrap().name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(0).unwrap().status);

                assert_eq!(rules.rules.get(1).unwrap().name, "rule2_sms");
                assert_eq!(ProcessedRuleStatus::NotMatched, rules.rules.get(1).unwrap().status);

                assert_eq!(rules.rules.get(2).unwrap().name, "rule3_email");
                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(2).unwrap().status);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_return_status_matched() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
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

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(1, rules.rules.len());

                let processed_rule = rules.rules.get(0).unwrap();
                assert_eq!(processed_rule.name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::Matched, processed_rule.status);
                assert_eq!(1, rules.extracted_vars.len());
                assert_eq!("ai", rules.extracted_vars.get("rule1_email.extracted_temp").unwrap());
                assert_eq!(1, processed_rule.actions.len());
                assert_eq!("ai", processed_rule.actions[0].payload.get("temp").unwrap());
                assert!(processed_rule.message.is_none())
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_return_status_not_matched_if_where_returns_false() {
        // Arrange
        let rule_1 = new_rule(
            "rule1_email",
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        // Act
        let result = matcher.process(Event::new("sms"));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(1, rules.rules.len());

                let processed_rule = rules.rules.get(0).unwrap();
                assert_eq!(processed_rule.name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::NotMatched, processed_rule.status);
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_return_status_partially_matched_if_extracted_var_is_missing() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.temp}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(1, rules.rules.len());

                let processed_rule = rules.rules.get(0).unwrap();
                assert_eq!(processed_rule.name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::PartiallyMatched, processed_rule.status);

                info!("Message: {:?}", processed_rule.message);
                assert!(processed_rule.message.clone().unwrap().contains("extracted_temp"))
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_return_status_partially_matched_if_action_payload_cannot_be_resolved() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
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

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset1".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        let mut event_payload = HashMap::new();
        event_payload.insert(String::from("temp"), Value::Text(String::from("temp_value")));

        // Act
        let result = matcher.process(Event::new_with_payload("email", event_payload));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset1", name);
                assert_eq!(1, rules.rules.len());

                let processed_rule = rules.rules.get(0).unwrap();
                assert_eq!(processed_rule.name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::PartiallyMatched, processed_rule.status);

                info!("Message: {:?}", processed_rule.message);
                assert!(processed_rule.message.clone().unwrap().contains("rule1_email.missing"))
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_stop_execution_if_continue_is_false() {
        // Arrange
        let op = Operator::Equal {
            first: Value::Text("${event.type}".to_owned()),
            second: Value::Text("email".to_owned()),
        };

        let rule_1 = new_rule("rule1_email", op.clone());

        let mut rule_2 = new_rule("rule2_email", op.clone());
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", op.clone());

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2, rule_3],
        })
        .unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(2, rules.rules.len());

                assert_eq!(rules.rules.get(0).unwrap().name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(0).unwrap().status);

                assert_eq!(rules.rules.get(1).unwrap().name, "rule2_email");
                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(1).unwrap().status);
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_not_stop_execution_if_continue_is_false_in_a_non_matching_rule() {
        // Arrange
        let op = Operator::Equal {
            first: Value::Text("${event.type}".to_owned()),
            second: Value::Text("email".to_owned()),
        };

        let rule_1 = new_rule("rule1_email", op.clone());

        let mut rule_2 = new_rule(
            "rule2_sms",
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("sms".to_owned()),
            },
        );
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", op.clone());

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2, rule_3],
        })
        .unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(3, rules.rules.len());

                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(0).unwrap().status);

                assert_eq!(ProcessedRuleStatus::NotMatched, rules.rules.get(1).unwrap().status);

                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(2).unwrap().status);
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_return_matching_rules_and_extracted_variables() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(1, rules.rules.len());

                let rule_1_processed = rules.rules.get(0).unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
                assert!(rules.extracted_vars.contains_key("rule1_email.extracted_temp"));
                assert_eq!("ai", rules.extracted_vars.get("rule1_email.extracted_temp").unwrap());
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_return_extracted_vars_grouped_by_rule() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
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
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
        );

        rule_2.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[em]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2],
        })
        .unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(2, rules.rules.len());

                let rule_1_processed = rules.rules.get(0).unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
                assert!(rules.extracted_vars.contains_key("rule1_email.extracted_temp"));
                assert_eq!("ai", rules.extracted_vars.get("rule1_email.extracted_temp").unwrap());

                let rule_2_processed = rules.rules.get(1).unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_2_processed.status);
                assert!(rules.extracted_vars.contains_key("rule2_email.extracted_temp"));
                assert_eq!("em", rules.extracted_vars.get("rule2_email.extracted_temp").unwrap());
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_return_rule_only_if_matches_the_extracted_variables_too() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
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
            Operator::Equal {
                first: Value::Text("${event.type}".to_owned()),
                second: Value::Text("email".to_owned()),
            },
        );

        rule_2.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex { regex: String::from(r"[ai]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2],
        })
        .unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(2, rules.rules.len());

                let rule_1_processed = rules.rules.get(0).unwrap();
                assert_eq!(ProcessedRuleStatus::PartiallyMatched, rule_1_processed.status);
                assert!(!rules.extracted_vars.contains_key("rule1_email.extracted_temp"));

                let rule_2_processed = rules.rules.get(1).unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_2_processed.status);
                assert!(rules.extracted_vars.contains_key("rule2_email.extracted_temp"));
                assert_eq!("ai", rules.extracted_vars.get("rule2_email.extracted_temp").unwrap());
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_match_rule_against_inner_array() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1",
            Operator::Equal {
                first: Value::Text("${event.payload.array[0]}".to_owned()),
                second: Value::Text("aaa".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.array[1]}"),
                regex: ExtractorRegex { regex: String::from(r"[z]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        let mut payload = Payload::new();
        payload.insert(
            "array".to_owned(),
            Value::Array(vec![Value::Text("aaa".to_owned()), Value::Text("zzz".to_owned())]),
        );

        // Act
        let result = matcher.process(Event::new_with_payload("email", payload));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                let rule_1_processed = rules.rules.get(0).unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
                assert_eq!(
                    "zzz",
                    rules.extracted_vars.get("rule1.extracted_temp").unwrap().get_text().unwrap()
                );
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_match_rule_against_inner_map() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1",
            Operator::Equal {
                first: Value::Text("${event.payload.map.key0}".to_owned()),
                second: Value::Text("aaa".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.map.key1}"),
                regex: ExtractorRegex { regex: String::from(r"[z]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        let mut payload = Payload::new();
        let mut inner = Payload::new();
        inner.insert("key0".to_owned(), Value::Text("aaa".to_owned()));
        inner.insert("key1".to_owned(), Value::Text("zzz".to_owned()));
        payload.insert("map".to_owned(), Value::Map(inner));

        // Act
        let result = matcher.process(Event::new_with_payload("email", payload));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                let rule_1_processed = rules.rules.get(0).unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
                assert_eq!(
                    "zzz",
                    rules.extracted_vars.get("rule1.extracted_temp").unwrap().get_text().unwrap()
                );
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_process_rulesets_if_filter_has_no_operator() {
        // Arrange
        let op = Operator::Equal {
            first: Value::Text("${event.type}".to_owned()),
            second: Value::Text("email".to_owned()),
        };

        let filter = new_filter(None);

        let nodes = vec![
            MatcherConfig::Ruleset {
                name: "node1".to_owned(),
                rules: vec![new_rule("rule_a1", op.clone())],
            },
            MatcherConfig::Ruleset {
                name: "node2".to_owned(),
                rules: vec![new_rule("rule_b1", op.clone())],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(2, nodes.len());

                match nodes.get(0).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node1", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.get(0).unwrap().name, "rule_a1");
                    }
                    _ => assert!(false),
                };

                match nodes.get(1).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node2", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.get(0).unwrap().name, "rule_b1");
                    }
                    _ => assert!(false),
                };
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_process_all_filter_rulesets() {
        // Arrange
        let op = Operator::Equal {
            first: Value::Text("${event.type}".to_owned()),
            second: Value::Text("email".to_owned()),
        };

        let filter = new_filter(op.clone());

        let nodes = vec![
            MatcherConfig::Ruleset {
                name: "node1".to_owned(),
                rules: vec![new_rule("rule_a1", None), new_rule("rule_a2", op.clone())],
            },
            MatcherConfig::Ruleset {
                name: "node2".to_owned(),
                rules: vec![new_rule("rule_b1", None), new_rule("rule_b2", op.clone())],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(2, nodes.len());

                match nodes.get(0).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node1", name);
                        assert_eq!(2, rules.rules.len());

                        assert_eq!(rules.rules.get(0).unwrap().name, "rule_a1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(0).unwrap().status
                        );

                        assert_eq!(rules.rules.get(1).unwrap().name, "rule_a2");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(1).unwrap().status
                        );
                    }
                    _ => assert!(false),
                };

                match nodes.get(1).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node2", name);
                        assert_eq!(2, rules.rules.len());

                        assert_eq!(rules.rules.get(0).unwrap().name, "rule_b1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(0).unwrap().status
                        );

                        assert_eq!(rules.rules.get(1).unwrap().name, "rule_b2");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(1).unwrap().status
                        );
                    }
                    _ => assert!(false),
                };
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_process_filter_rulesets_recursively() {
        // Arrange
        let op = Operator::Equal {
            first: Value::Text("${event.type}".to_owned()),
            second: Value::Text("email".to_owned()),
        };

        let filter = new_filter(op.clone());

        let nodes = vec![
            MatcherConfig::Filter {
                name: "node0".to_owned(),
                filter: new_filter(op.clone()),
                nodes: vec![MatcherConfig::Ruleset {
                    name: "node".to_owned(),
                    rules: vec![new_rule("rule2", None)],
                }],
            },
            MatcherConfig::Filter {
                name: "node1".to_owned(),
                filter: new_filter(Operator::Equal {
                    first: Value::Text("${event.type}".to_owned()),
                    second: Value::Text("trap".to_owned()),
                }),
                nodes: vec![MatcherConfig::Ruleset {
                    name: "node".to_owned(),
                    rules: vec![new_rule("rule3", None)],
                }],
            },
            MatcherConfig::Ruleset {
                name: "node2".to_owned(),
                rules: vec![new_rule("rule_a1", None), new_rule("rule_a2", op.clone())],
            },
            MatcherConfig::Ruleset {
                name: "node3".to_owned(),
                rules: vec![new_rule("rule_b1", None), new_rule("rule_b2", op.clone())],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter1".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!(name, "filter1");
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(4, nodes.len());

                match nodes.get(0).unwrap() {
                    ProcessedNode::Filter { name, filter, nodes } => {
                        assert_eq!(name, "node0");
                        assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                        assert_eq!(1, nodes.len());

                        match nodes.get(0).unwrap() {
                            ProcessedNode::Ruleset { name, rules } => {
                                assert_eq!(name, "node");
                                assert_eq!(1, rules.rules.len());
                                assert_eq!(rules.rules.get(0).unwrap().name, "rule2");
                                assert_eq!(
                                    ProcessedRuleStatus::Matched,
                                    rules.rules.get(0).unwrap().status
                                );
                            }
                            _ => assert!(false),
                        };
                    }
                    _ => assert!(false),
                };

                match nodes.get(1).unwrap() {
                    ProcessedNode::Filter { name, filter, nodes } => {
                        assert_eq!(name, "node1");
                        assert_eq!(ProcessedFilterStatus::NotMatched, filter.status);
                        assert_eq!(0, nodes.len());
                    }
                    _ => assert!(false),
                };

                match nodes.get(2).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!(name, "node2");
                        assert_eq!(2, rules.rules.len());
                        assert_eq!(rules.rules.get(0).unwrap().name, "rule_a1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(0).unwrap().status
                        );
                        assert_eq!(rules.rules.get(1).unwrap().name, "rule_a2");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(1).unwrap().status
                        );
                    }
                    _ => assert!(false),
                };

                match nodes.get(3).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!(name, "node3");
                        assert_eq!(2, rules.rules.len());
                        assert_eq!(rules.rules.get(0).unwrap().name, "rule_b1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(0).unwrap().status
                        );
                        assert_eq!(rules.rules.get(1).unwrap().name, "rule_b2");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(1).unwrap().status
                        );
                    }
                    _ => assert!(false),
                };
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_process_no_rulesets_if_filter_is_inactive() {
        // Arrange
        let op = Operator::Equal {
            first: Value::Text("${event.type}".to_owned()),
            second: Value::Text("email".to_owned()),
        };

        let mut filter = new_filter(None);
        filter.active = false;

        let nodes = vec![
            MatcherConfig::Ruleset {
                name: "node0".to_owned(),
                rules: vec![new_rule("rule_a1", op.clone())],
            },
            MatcherConfig::Ruleset {
                name: "node1".to_owned(),
                rules: vec![new_rule("rule_b1", op.clone())],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::Inactive, filter.status);
                assert_eq!(0, nodes.len());
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_process_no_rulesets_if_filter_does_not_match() {
        // Arrange
        let op = Operator::Equal {
            first: Value::Text("${event.type}".to_owned()),
            second: Value::Text("email".to_owned()),
        };

        let filter = new_filter(Operator::Equal {
            first: Value::Text("${event.type}".to_owned()),
            second: Value::Text("trapd".to_owned()),
        });

        let nodes = vec![
            MatcherConfig::Ruleset {
                name: "node0".to_owned(),
                rules: vec![new_rule("rule_a1", op.clone()), new_rule("rule_a2", op.clone())],
            },
            MatcherConfig::Ruleset {
                name: "node1".to_owned(),
                rules: vec![new_rule("rule_b1", op.clone()), new_rule("rule_b2", op.clone())],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::NotMatched, filter.status);
                assert_eq!(0, nodes.len());
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_process_rulesets_independently() {
        // Arrange
        let op = Operator::Equal {
            first: Value::Text("${event.type}".to_owned()),
            second: Value::Text("email".to_owned()),
        };

        let filter = new_filter(op.clone());

        let mut rule_a1 = new_rule("rule_a1", None);
        rule_a1.do_continue = false;

        let mut rule_b1 = new_rule("rule_b1", None);
        rule_b1.do_continue = false;

        let mut rule_c1 = new_rule("rule_c1", None);
        rule_c1.do_continue = false;

        let nodes = vec![
            MatcherConfig::Ruleset {
                name: "node0".to_owned(),
                rules: vec![rule_a1, new_rule("rule_a2", op.clone())],
            },
            MatcherConfig::Ruleset {
                name: "node1".to_owned(),
                rules: vec![rule_b1, new_rule("rule_b2", op.clone())],
            },
            MatcherConfig::Ruleset {
                name: "node2".to_owned(),
                rules: vec![rule_c1, new_rule("rule_c2", op.clone())],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(Event::new("email"));

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(3, nodes.len());

                match nodes.get(0).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node0", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.get(0).unwrap().name, "rule_a1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(0).unwrap().status
                        );
                    }
                    _ => assert!(false),
                };

                match nodes.get(1).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node1", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.get(0).unwrap().name, "rule_b1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(0).unwrap().status
                        );
                    }
                    _ => assert!(false),
                };

                match nodes.get(2).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node2", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.get(0).unwrap().name, "rule_c1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(0).unwrap().status
                        );
                    }
                    _ => assert!(false),
                };
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn extracted_variables_should_be_independent_for_each_pipeline() {
        // Arrange
        let mut rule_0 = new_rule("rule", None);

        rule_0.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.value}"),
                regex: ExtractorRegex { regex: String::from(r"[a-z]+"), group_match_idx: 0 },
            },
        );

        let mut rule_1 = new_rule("rule", None);

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.value}"),
                regex: ExtractorRegex { regex: String::from(r"[0-9]+"), group_match_idx: 0 },
            },
        );

        let filter = new_filter(None);

        let nodes = vec![
            MatcherConfig::Ruleset { name: "node0".to_owned(), rules: vec![rule_0] },
            MatcherConfig::Ruleset { name: "node1".to_owned(), rules: vec![rule_1] },
        ];

        let matcher =
            new_matcher(&MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes })
                .unwrap();

        let mut payload = Payload::new();
        payload.insert("value".to_owned(), Value::Text("aaa999".to_owned()));

        // Act
        let result = matcher.process(Event::new_with_payload("email", payload));

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(2, nodes.len());

                match nodes.get(0).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node0", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.get(0).unwrap().name, "rule");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(0).unwrap().status
                        );
                        assert_eq!("aaa", rules.extracted_vars.get("rule.extracted_temp").unwrap());
                    }
                    _ => assert!(false),
                };

                match nodes.get(1).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node1", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.get(0).unwrap().name, "rule");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(0).unwrap().status
                        );
                        assert_eq!("999", rules.extracted_vars.get("rule.extracted_temp").unwrap());
                    }
                    _ => assert!(false),
                };
            }
            _ => assert!(false),
        };
    }

    #[test]
    fn should_match_cmp_operators() {
        // Arrange
        let filename = "./test_resources/rules/004_cmp_operators.json";
        let json = std::fs::read_to_string(filename)
            .expect(&format!("Unable to open the file [{}]", filename));
        let mut rule = Rule::from_json(&json).unwrap();
        rule.name = "cmp_operators".to_owned();

        let mut payload = Payload::new();
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule.clone()],
        })
        .unwrap();

        // Value equal to 1000 should match
        {
            // Act
            payload.insert("value".to_owned(), Value::Number(Number::PosInt(1000)));
            let result = matcher.process(Event::new_with_payload("email", payload.clone()));

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.get(0).unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(0).unwrap().status);
                }
                _ => assert!(false),
            };
        }

        // Value equal to 2000 should not match
        {
            // Act
            payload.insert("value".to_owned(), Value::Number(Number::PosInt(2000)));
            let result = matcher.process(Event::new_with_payload("email", payload.clone()));

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.get(0).unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::NotMatched, rules.rules.get(0).unwrap().status);
                }
                _ => assert!(false),
            };
        }

        // Value less than 0 should match
        {
            // Act
            payload.insert("value".to_owned(), Value::Number(Number::NegInt(-1000)));
            let result = matcher.process(Event::new_with_payload("email", payload.clone()));

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.get(0).unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(0).unwrap().status);
                }
                _ => assert!(false),
            };
        }

        // Value more than 2000 (not included) should match
        {
            // Act
            payload.insert("value".to_owned(), Value::Number(Number::Float(1000000000.0)));
            let result = matcher.process(Event::new_with_payload("email", payload.clone()));

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.get(0).unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(0).unwrap().status);
                }
                _ => assert!(false),
            };
        }

        // Value between 100 and 200 (included) should match
        {
            // Act
            payload.insert("value".to_owned(), Value::Number(Number::PosInt(100)));
            let result = matcher.process(Event::new_with_payload("email", payload.clone()));

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.get(0).unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(0).unwrap().status);
                }
                _ => assert!(false),
            };
        }

        // Value between 100 and 200 (included) should match
        {
            // Act
            payload.insert("value".to_owned(), Value::Number(Number::PosInt(150)));
            let result = matcher.process(Event::new_with_payload("email", payload.clone()));

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.get(0).unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(0).unwrap().status);
                }
                _ => assert!(false),
            };
        }

        // Value between 100 and 200 (included) should match
        {
            // Act
            payload.insert("value".to_owned(), Value::Number(Number::PosInt(200)));
            let result = matcher.process(Event::new_with_payload("email", payload.clone()));

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.get(0).unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(0).unwrap().status);
                }
                _ => assert!(false),
            };
        }
    }

    #[test]
    fn rule_should_get_extracted_variables_of_another_rule() {
        // Arrange
        let mut rule_1 = new_rule("rule1", None);

        rule_1.constraint.with.insert(
            String::from("extracted"),
            Extractor {
                from: String::from("${event.payload.value}"),
                regex: ExtractorRegex { regex: String::from(r"[a-z]+"), group_match_idx: 0 },
            },
        );

        let mut rule_2 = new_rule(
            "rule2",
            Operator::Equal {
                first: Value::Text("${_variables.rule1.extracted}".to_owned()),
                second: Value::Text("aaa".to_owned()),
            },
        );

        rule_2.constraint.with.insert(
            String::from("extracted"),
            Extractor {
                from: String::from("${event.payload.value}"),
                regex: ExtractorRegex { regex: String::from(r"[0-9]+"), group_match_idx: 0 },
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2],
        })
        .expect("should create a matcher");

        let mut payload = Payload::new();
        payload.insert("value".to_owned(), Value::Text("aaa999".to_owned()));

        // Act
        let result = matcher.process(Event::new_with_payload("email", payload));

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(2, rules.rules.len());

                assert_eq!(
                    "aaa",
                    rules
                        .extracted_vars
                        .get("rule1.extracted")
                        .expect("should contain rule1.extracted")
                );
                assert_eq!(
                    "999",
                    rules
                        .extracted_vars
                        .get("rule2.extracted")
                        .expect("should contain rule2.extracted")
                );

                let rule_1_processed = rules.rules.get(0).expect("should contain rule1");
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);

                let rule_2_processed = rules.rules.get(1).expect("should contain rule2");
                assert_eq!(ProcessedRuleStatus::Matched, rule_2_processed.status);
            }
            _ => assert!(false),
        };
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

    fn new_filter<O: Into<Option<Operator>>>(filter: O) -> Filter {
        Filter { active: true, description: "".to_owned(), filter: filter.into() }
    }
}
