pub mod action;
pub mod extractor;
pub mod modifier;
pub mod operator;

use tracing::instrument;

use crate::accessor::AccessorBuilder;
use crate::config::rule::Rule;
use crate::config::MatcherConfig;
use crate::error::MatcherError;
use crate::matcher::extractor::{MatcherExtractor, MatcherExtractorBuilder};
use crate::model::{
    InternalEvent, ProcessedEvent, ProcessedFilter, ProcessedFilterStatus, ProcessedIteration,
    ProcessedIterator, ProcessedNode, ProcessedRule, ProcessedRuleMetaData, ProcessedRuleStatus,
    ProcessedRules,
};
use crate::validator::MatcherConfigValidator;
use log::*;
use serde_json::{json, Map, Value};
use tornado_common_parser::AccessorExpression;

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
    Iterator { name: String, target: AccessorExpression, nodes: Vec<ProcessingNode> },
    Ruleset { name: String, rules: Vec<MatcherRule> },
}

/// The Matcher contains the core logic of the Tornado Engine.
/// It matches incoming Events against the defined Rules.
/// A Matcher instance is stateless and thread-safe; consequently, a single instance can serve the entire application.
pub struct Matcher {
    node: ProcessingNode,
}

fn build_matcher_rule(rule: &Rule) -> Result<MatcherRule, MatcherError> {
    let action_builder = action::ActionResolverBuilder::new();
    let operator_builder = operator::OperatorBuilder::new();
    let extractor_builder = MatcherExtractorBuilder::new();

    debug!("Matcher build - Processing rule: [{}]", &rule.name);
    trace!("Matcher build - Processing rule definition:\n{:?}", rule);

    Ok(MatcherRule {
        name: rule.name.to_owned(),
        do_continue: rule.do_continue,
        operator: operator_builder.build_option(&rule.name, &rule.constraint.where_operator)?,
        extractor: extractor_builder.build(&rule.name, &rule.constraint.with)?,
        actions: action_builder.build_all(&rule.name, &rule.actions)?,
    })
}

pub fn validate_rule(rule: &Rule) -> Result<(), MatcherError> {
    let _ = build_matcher_rule(rule)?;
    Ok(())
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
                let processed_rules = rules
                    .iter()
                    .filter(|rule| rule.active)
                    .map(build_matcher_rule)
                    .collect::<Result<_, _>>()?;

                info!("Matcher Rules build completed");

                Ok(ProcessingNode::Ruleset { name: name.to_owned(), rules: processed_rules })
            }
            MatcherConfig::Filter { name, filter, nodes } => {
                debug!("Start processing Matcher Filter [{}] Config", name);
                let operator_builder = operator::OperatorBuilder::new();

                let matcher_filter = MatcherFilter {
                    active: filter.active,
                    filter: operator_builder.build_option(name, &filter.filter.clone().into())?,
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
            MatcherConfig::Iterator { name, iterator, nodes } => {
                let builder = AccessorBuilder::new();
                let exp = builder.build(name, iterator.target())?.try_as_expression()?;
                let children = nodes
                    .iter()
                    .filter(|_| iterator.is_active())
                    .map(Matcher::build_processing_tree)
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(ProcessingNode::Iterator { name: name.clone(), target: exp, nodes: children })
            }
        }
    }

    /// Processes an incoming Event and compares it against the set of Rules defined at the Matcher's creation time.
    /// The result is a ProcessedEvent.
    pub fn process(&self, event: Value, include_metadata: bool) -> ProcessedEvent {
        trace!(
            "Matcher process - processing event: [{:?}], include metadata: [{}]",
            &event,
            include_metadata
        );
        let result = Matcher::process_node(&self.node, &event, include_metadata);
        ProcessedEvent { event, result }
    }

    fn process_node(
        node: &ProcessingNode,
        internal_event: &Value,
        include_metadata: bool,
    ) -> ProcessedNode {
        match node {
            ProcessingNode::Filter { name, filter, nodes } => {
                Matcher::process_filter(name, filter, nodes, internal_event, include_metadata)
            }
            ProcessingNode::Ruleset { name, rules } => {
                Matcher::process_rules(name, rules, internal_event, include_metadata)
            }
            ProcessingNode::Iterator { name, target, nodes } => {
                Matcher::process_iterator(name, target, nodes, internal_event, include_metadata)
            }
        }
    }

    #[instrument(level = "debug", skip_all, fields(otel.name = format ! ("Process Filter: {}", filter_name).as_str()))]
    fn process_filter(
        filter_name: &str,
        filter: &MatcherFilter,
        nodes: &[ProcessingNode],
        event: &Value,
        include_metadata: bool,
    ) -> ProcessedNode {
        trace!("Matcher process - check matching of filter: [{}]", filter_name);

        let mut result_nodes = vec![];

        let filter_status = if filter.active {
            let internal_event = InternalEvent { event, extracted_variables: &mut Value::Null };
            if filter.filter.evaluate(&internal_event) {
                trace!(
                        "Matcher process - event matches filter: [{}]. Passing the Event to the nested nodes.",
                        filter_name
                    );
                nodes.iter().for_each(|node| {
                    let processed_node = Matcher::process_node(node, event, include_metadata);
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

    #[instrument(level = "debug", skip_all, fields(otel.name = format ! ("Process Iterator: {}", name).as_str()))]
    fn process_iterator(
        name: &str,
        target: &AccessorExpression,
        nodes: &[ProcessingNode],
        event: &Value,
        include_metadata: bool,
    ) -> ProcessedNode {
        trace!("Matcher process - check matching of iterator: [{}]", name);
        let internal_event = InternalEvent { event, extracted_variables: &mut Default::default() };
        let Some(target) = target.parse_value(&internal_event) else {
            // ToDo: Improve in NEPROD-1682
            return ProcessedNode::Iterator {
                name: name.to_string(),
                iterator: ProcessedIterator::AccessorError,
                events: vec![],
            };
        };

        match target.as_ref() {
            Value::Array(slice) => {
                let iterator = slice.iter().enumerate();
                Matcher::iterate_over(name, iterator, event, nodes, include_metadata)
            }
            Value::Object(map) => {
                let iterator =
                    map.keys().flat_map(|key| map.get(key).map(|value| (key.as_str(), value)));
                Matcher::iterate_over(name, iterator, event, nodes, include_metadata)
            }
            _ => {
                return ProcessedNode::Iterator {
                    name: name.to_string(),
                    iterator: ProcessedIterator::TypeError,
                    events: vec![],
                }
            }
        }
    }

    fn iterate_over<'a, Key, Iter>(
        name: &str,
        iterator: Iter,
        event: &Value,
        nodes: &[ProcessingNode],
        include_metadata: bool,
    ) -> ProcessedNode
    where
        Key: Into<Value> + Copy,
        Iter: Iterator<Item = (Key, &'a Value)>,
    {
        let mut processed_events = vec![];
        let mut iterator_event = event.clone();
        for (iteration, item) in iterator {
            let Some(event_inner) = iterator_event.as_object_mut() else {
                continue;
            };

            let Some(iterator) = event_inner.entry("iterator").or_insert(json!({})).as_object_mut()
            else {
                // we just inserted it as an iterator. This path will not be taken.
                continue;
            };

            iterator.insert("iteration".to_string(), iteration.into());
            iterator.insert("item".to_string(), item.clone());

            let mut processed_nodes = vec![];
            for node in nodes {
                let processed_node = Matcher::process_node(node, &iterator_event, include_metadata);
                processed_nodes.push(processed_node)
            }

            if include_metadata {
                processed_events
                    .push(ProcessedIteration { event: iterator_event, result: processed_nodes });
                iterator_event = event.clone();
            } else {
                processed_events
                    .push(ProcessedIteration { event: Value::Null, result: processed_nodes });
            }
        }

        ProcessedNode::Iterator {
            name: name.to_string(),
            iterator: ProcessedIterator::Matched,
            events: processed_events,
        }
    }

    #[instrument(level = "debug", skip_all, fields(otel.name = format ! ("Process Ruleset: {}", ruleset_name).as_str()))]
    fn process_rules(
        ruleset_name: &str,
        rules: &[MatcherRule],
        event: &Value,
        include_metadata: bool,
    ) -> ProcessedNode {
        trace!("Matcher process - check matching of ruleset: [{}]", ruleset_name);
        let mut extracted_vars = Value::Object(Map::new());
        let mut internal_event = InternalEvent { event, extracted_variables: &mut extracted_vars };

        let mut processed_rules = vec![];

        for rule in rules {
            let _rule_span = tracing::debug_span!(
                "process_rule",
                name = rule.name.as_str(),
                otel.name = format!("Process Rule: {}", rule.name).as_str()
            )
            .entered();
            trace!("Matcher process - check matching of rule: [{}]", &rule.name);

            let mut processed_rule = ProcessedRule {
                name: rule.name.clone(),
                status: ProcessedRuleStatus::NotMatched,
                actions: vec![],
                message: None,
                meta: None,
            };

            if include_metadata {
                processed_rule.meta = Some(ProcessedRuleMetaData { actions: vec![] })
            }

            if rule.operator.evaluate(&internal_event) {
                trace!(
                    "Matcher process - event matches rule: [{}]. Checking extracted variables.",
                    &rule.name
                );

                match rule.extractor.process_all(&mut internal_event) {
                    Ok(_) => {
                        trace!("Matcher process - event matches rule: [{}] and its extracted variables.", &rule.name);

                        match Matcher::process_actions(
                            &internal_event,
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
                                let message = format!("Matcher process - The event matches the rule [{}] and all variables are extracted correctly; however, some actions cannot be resolved: [{:?}]", &rule.name, e);
                                debug!("{}", &message);
                                processed_rule.status = ProcessedRuleStatus::PartiallyMatched;
                                processed_rule.message = Some(message);
                            }
                        }
                    }
                    Err(e) => {
                        let message = format!("Matcher process - The event matches the rule [{}] but some variables cannot be extracted: [{:?}]", &rule.name, e);
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
        processed_rule: &mut ProcessedRule,
        actions: &[action::ActionResolver],
    ) -> Result<(), MatcherError> {
        for action in actions {
            let _action_span = tracing::debug_span!(
                "process_action",
                otel.name = format!("Process Action: {}", action.id).as_str()
            )
            .entered();

            if let Some(metadata) = &mut processed_rule.meta {
                let (action, action_metadata) = action.resolve_with_meta(processed_event)?;
                processed_rule.actions.push(action);
                metadata.actions.push(action_metadata);
            } else {
                processed_rule.actions.push(action.resolve(processed_event)?);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::nodes::{Filter, MatcherIterator};
    use crate::config::rule::{
        ConfigAction, Constraint, Extractor, ExtractorRegex, Operator, Rule,
    };
    use crate::config::v2::{parse_from_file, FsMatcherConfigManagerV2};
    use crate::config::{Defaultable, MatcherConfigReader};
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::Path;
    use tornado_common_api::{Action, Event, Payload, ValueExt, ValueGet};

    #[test]
    fn should_build_the_matcher_with_a_rule_set() {
        // Arrange
        let rule = new_rule(
            "rule_name",
            Operator::Equals {
                first: Value::String("1".to_owned()),
                second: Value::String("1".to_owned()),
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
            _ => unreachable!(),
        }
    }

    #[test]
    fn should_build_the_matcher_with_a_filter() {
        // Arrange
        let filter = new_filter(Operator::Equals {
            first: Value::String("1".to_owned()),
            second: Value::String("1".to_owned()),
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
            _ => unreachable!(),
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
            _ => unreachable!(),
        }
    }

    #[test]
    fn should_build_the_matcher_with_a_filter_recursively() {
        // Arrange
        let filter = new_filter(Operator::Equals {
            first: Value::String("1".to_owned()),
            second: Value::String("1".to_owned()),
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

                match &nodes1.first().unwrap() {
                    ProcessingNode::Filter { name, filter: _filter2, nodes: nodes2 } => {
                        assert_eq!(1, nodes2.len());
                        assert_eq!("node1", name);

                        match &nodes2.first().unwrap() {
                            ProcessingNode::Ruleset { rules: rules2, .. } => {
                                assert_eq!(1, rules2.len());
                                assert_eq!("rule2", rules2.first().unwrap().name);
                            }
                            _ => unreachable!(),
                        }
                    }
                    _ => unreachable!(),
                }

                match &nodes1.get(1).unwrap() {
                    ProcessingNode::Ruleset { name, rules: rules1 } => {
                        assert_eq!("node2", name);
                        assert_eq!(1, rules1.len());
                        assert_eq!("rule1", rules1.first().unwrap().name);
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
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
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2],
        });

        // Assert
        assert!(matcher.is_err());

        match matcher.err().unwrap() {
            MatcherError::NotUniqueNameError { name } => assert_eq!("rule_name", name),
            _ => unreachable!(),
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
            _ => unreachable!(),
        }
    }

    #[test]
    fn should_build_a_iterator_processing_node() {
        let config = MatcherConfig::Iterator {
            name: "master_iterator".to_string(),
            iterator: MatcherIterator {
                description: "...".to_string(),
                active: true,
                target: "${event.payload}".to_string(),
            },
            nodes: vec![],
        };

        let matcher = Matcher::build_processing_tree(&config).unwrap();

        match matcher {
            ProcessingNode::Iterator { target, .. } => {
                assert_eq!(2, target.keys.len());
            }
            _ => unreachable!(),
        };
    }

    #[tokio::test]
    async fn should_process_data() {
        let config = FsMatcherConfigManagerV2::new("./test_resources/v2/test_config_iterator/", "")
            .get_config()
            .await
            .unwrap();

        let processing_node = Matcher::build(&config).unwrap();

        let test_event: Value =
            parse_from_file(Path::new("./test_resources/test_events/openshift_webhook_event.json"))
                .await
                .unwrap();

        let result = processing_node.process(test_event, true);

        let master = match &result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("root", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(1, nodes.len());
                &nodes[0]
            }
            result => unreachable!("{:?}", result),
        };

        let iterator_filter = match master {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("master", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(1, nodes.len());
                &nodes[0]
            }
            result => unreachable!("{:?}", result),
        };

        let iterator = match iterator_filter {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("prometheus_alert_manager", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(1, nodes.len());
                &nodes[0]
            }
            result => unreachable!("{:?}", result),
        };

        let [event1, event2, event3, event4, event5] = match iterator {
            ProcessedNode::Iterator { name, iterator, events } => {
                assert_eq!("openshift_iterator", name);
                assert_eq!(&ProcessedIterator::Matched, iterator);
                assert_eq!(5, events.len());
                [&events[0], &events[1], &events[2], &events[3], &events[4]]
            }
            result => unreachable!("{:?}", result),
        };

        assert_eq!(0, event1.event["iterator"]["iteration"]);
        assert_eq!(1, event2.event["iterator"]["iteration"]);
        assert_eq!(2, event3.event["iterator"]["iteration"]);
        assert_eq!(3, event4.event["iterator"]["iteration"]);
        assert_eq!(4, event5.event["iterator"]["iteration"]);

        fn collect_actions(result: ProcessedNode) -> Vec<Action> {
            match result {
                ProcessedNode::Filter { nodes, .. } => {
                    nodes.into_iter().flat_map(collect_actions).collect()
                }
                ProcessedNode::Iterator { events, .. } => {
                    events.into_iter().flat_map(|e| e.result).flat_map(collect_actions).collect()
                }
                ProcessedNode::Ruleset { rules, .. } => {
                    rules.rules.into_iter().flat_map(|r| r.actions).collect()
                }
            }
        }

        let actions = collect_actions(result.result);
        assert_eq!(15, actions.len());
    }

    #[test]
    fn should_sort_the_rules_based_on_input_order() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("1".to_owned()),
            second: Value::String("1".to_owned()),
        };
        let rule_1 = new_rule("rule1", op.clone());
        let rule_2 = new_rule("rule2", op.clone());
        let rule_3 = new_rule("rule3", op.clone());
        let rule_4 = new_rule("rule4", op);

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
            _ => unreachable!(),
        }
    }

    #[test]
    fn should_ignore_non_active_rules() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("1".to_owned()),
            second: Value::String("1".to_owned()),
        };
        let mut rule_1 = new_rule("rule1", op.clone());
        rule_1.active = false;

        let rule_2 = new_rule("rule2", op.clone());

        let mut rule_3 = new_rule("rule3", op.clone());
        rule_3.active = false;

        let rule_4 = new_rule("rule4", op);

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
            _ => unreachable!(),
        }
    }

    #[test]
    fn should_return_matching_rules() {
        // Arrange
        let rule_1 = new_rule(
            "rule1_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        let rule_2 = new_rule(
            "rule2_sms",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("sms".to_owned()),
            },
        );

        let rule_3 = new_rule(
            "rule3_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2, rule_3],
        })
        .unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(3, rules.rules.len());

                assert_eq!(rules.rules.first().unwrap().name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);

                assert_eq!(rules.rules.get(1).unwrap().name, "rule2_sms");
                assert_eq!(ProcessedRuleStatus::NotMatched, rules.rules.get(1).unwrap().status);

                assert_eq!(rules.rules.get(2).unwrap().name, "rule3_email");
                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(2).unwrap().status);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn should_return_status_matched() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[ai]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let mut action = ConfigAction { id: String::from("action_id"), payload: Map::new() };

        action
            .payload
            .insert("temp".to_owned(), Value::String("${_variables.extracted_temp}".to_owned()));
        rule_1.actions.push(action);

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(1, rules.rules.len());

                let processed_rule = rules.rules.first().unwrap();
                assert_eq!(processed_rule.name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::Matched, processed_rule.status);
                assert_eq!(1, rules.extracted_vars.get_map().unwrap().len());
                assert_eq!(
                    "ai",
                    rules
                        .extracted_vars
                        .get_from_map("rule1_email")
                        .unwrap()
                        .get_from_map("extracted_temp")
                        .unwrap()
                );
                assert_eq!(1, processed_rule.actions.len());
                assert_eq!("ai", processed_rule.actions[0].payload.get("temp").unwrap());
                assert!(processed_rule.message.is_none())
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_return_status_not_matched_if_where_returns_false() {
        // Arrange
        let rule_1 = new_rule(
            "rule1_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        // Act
        let result = matcher.process(json!(Event::new("sms")), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(1, rules.rules.len());

                let processed_rule = rules.rules.first().unwrap();
                assert_eq!(processed_rule.name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::NotMatched, processed_rule.status);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_return_status_partially_matched_if_extracted_var_is_missing() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.temp}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[ai]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(1, rules.rules.len());

                let processed_rule = rules.rules.first().unwrap();
                assert_eq!(processed_rule.name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::PartiallyMatched, processed_rule.status);

                info!("Message: {:?}", processed_rule.message);
                assert!(processed_rule.message.clone().unwrap().contains("extracted_temp"))
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_return_status_partially_matched_if_action_payload_cannot_be_resolved() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.temp}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[ai]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let mut action = ConfigAction { id: String::from("action_id"), payload: Map::new() };

        action
            .payload
            .insert("temp".to_owned(), Value::String("${_variables.extracted_temp}".to_owned()));
        action
            .payload
            .insert("missing".to_owned(), Value::String("${_variables.missing}".to_owned()));
        rule_1.actions.push(action);

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset1".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        let mut event_payload = Map::new();
        event_payload.insert(String::from("temp"), Value::String(String::from("temp_value")));

        // Act
        let result = matcher.process(json!(Event::new_with_payload("email", event_payload)), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset1", name);
                assert_eq!(1, rules.rules.len());

                let processed_rule = rules.rules.first().unwrap();
                assert_eq!(processed_rule.name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::PartiallyMatched, processed_rule.status);

                assert!(processed_rule.message.clone().unwrap().contains(
                    r#"parser: Exp(AccessorExpression { keys: [Map { key: \"missing\" }] })"#
                ))
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_stop_execution_if_continue_is_false() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("${event.type}".to_owned()),
            second: Value::String("email".to_owned()),
        };

        let rule_1 = new_rule("rule1_email", op.clone());

        let mut rule_2 = new_rule("rule2_email", op.clone());
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", op);

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2, rule_3],
        })
        .unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(2, rules.rules.len());

                assert_eq!(rules.rules.first().unwrap().name, "rule1_email");
                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);

                assert_eq!(rules.rules.get(1).unwrap().name, "rule2_email");
                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(1).unwrap().status);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_not_stop_execution_if_continue_is_false_in_a_non_matching_rule() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("${event.type}".to_owned()),
            second: Value::String("email".to_owned()),
        };

        let rule_1 = new_rule("rule1_email", op.clone());

        let mut rule_2 = new_rule(
            "rule2_sms",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("sms".to_owned()),
            },
        );
        rule_2.do_continue = false;

        let rule_3 = new_rule("rule3_email", op);

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2, rule_3],
        })
        .unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(3, rules.rules.len());

                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);

                assert_eq!(ProcessedRuleStatus::NotMatched, rules.rules.get(1).unwrap().status);

                assert_eq!(ProcessedRuleStatus::Matched, rules.rules.get(2).unwrap().status);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_return_matching_rules_and_extracted_variables() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[ai]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(1, rules.rules.len());

                let rule_1_processed = rules.rules.first().unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
                assert_eq!(
                    "ai",
                    rules
                        .extracted_vars
                        .get_from_map("rule1_email")
                        .unwrap()
                        .get_from_map("extracted_temp")
                        .unwrap()
                );
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_return_extracted_vars_grouped_by_rule() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[ai]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let mut rule_2 = new_rule(
            "rule2_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        rule_2.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[em]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2],
        })
        .unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(2, rules.rules.len());

                let rule_1_processed = rules.rules.first().unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
                assert_eq!(
                    "ai",
                    rules
                        .extracted_vars
                        .get_from_map("rule1_email")
                        .unwrap()
                        .get_from_map("extracted_temp")
                        .unwrap()
                );

                let rule_2_processed = rules.rules.get(1).unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_2_processed.status);
                assert_eq!(
                    "em",
                    rules
                        .extracted_vars
                        .get_from_map("rule2_email")
                        .unwrap()
                        .get_from_map("extracted_temp")
                        .unwrap()
                );
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_return_rule_only_if_matches_the_extracted_variables_too() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[z]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let mut rule_2 = new_rule(
            "rule2_email",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        rule_2.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[ai]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2],
        })
        .unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(2, rules.rules.len());

                let rule_1_processed = rules.rules.first().unwrap();
                assert_eq!(ProcessedRuleStatus::PartiallyMatched, rule_1_processed.status);
                assert!(rules
                    .extracted_vars
                    .get_from_map("rule1_email")
                    .and_then(|inner| inner.get_from_map("extracted_temp"))
                    .is_none());

                let rule_2_processed = rules.rules.get(1).unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_2_processed.status);
                assert_eq!(
                    "ai",
                    rules
                        .extracted_vars
                        .get_from_map("rule2_email")
                        .unwrap()
                        .get_from_map("extracted_temp")
                        .unwrap()
                );
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_match_rule_against_inner_array() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1",
            Operator::Equals {
                first: Value::String("${event.payload.array[0]}".to_owned()),
                second: Value::String("aaa".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.array[1]}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[z]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
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
            Value::Array(vec![Value::String("aaa".to_owned()), Value::String("zzz".to_owned())]),
        );

        // Act
        let result = matcher.process(json!(Event::new_with_payload("email", payload)), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                let rule_1_processed = rules.rules.first().unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
                assert_eq!(
                    "zzz",
                    rules
                        .extracted_vars
                        .get_from_map("rule1")
                        .unwrap()
                        .get_from_map("extracted_temp")
                        .unwrap()
                        .get_text()
                        .unwrap()
                );
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_match_rule_against_inner_map() {
        // Arrange
        let mut rule_1 = new_rule(
            "rule1",
            Operator::Equals {
                first: Value::String("${event.payload.map.key0}".to_owned()),
                second: Value::String("aaa".to_owned()),
            },
        );

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.map.key1}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[z]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1],
        })
        .unwrap();

        let mut payload = Payload::new();
        let mut inner = Payload::new();
        inner.insert("key0".to_owned(), Value::String("aaa".to_owned()));
        inner.insert("key1".to_owned(), Value::String("zzz".to_owned()));
        payload.insert("map".to_owned(), Value::Object(inner));

        // Act
        let result = matcher.process(json!(Event::new_with_payload("email", payload)), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                let rule_1_processed = rules.rules.first().unwrap();
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);
                assert_eq!(
                    "zzz",
                    rules
                        .extracted_vars
                        .get_from_map("rule1")
                        .unwrap()
                        .get_from_map("extracted_temp")
                        .unwrap()
                        .get_text()
                        .unwrap()
                );
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_process_rulesets_if_filter_has_no_operator() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("${event.type}".to_owned()),
            second: Value::String("email".to_owned()),
        };

        let filter = new_filter(None);

        let nodes = vec![
            MatcherConfig::Ruleset {
                name: "node1".to_owned(),
                rules: vec![new_rule("rule_a1", op.clone())],
            },
            MatcherConfig::Ruleset {
                name: "node2".to_owned(),
                rules: vec![new_rule("rule_b1", op)],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(2, nodes.len());

                match nodes.first().unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node1", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.first().unwrap().name, "rule_a1");
                    }
                    _ => unreachable!(),
                };

                match nodes.get(1).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node2", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.first().unwrap().name, "rule_b1");
                    }
                    _ => unreachable!(),
                };
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_process_all_filter_rulesets() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("${event.type}".to_owned()),
            second: Value::String("email".to_owned()),
        };

        let filter = new_filter(op.clone());

        let nodes = vec![
            MatcherConfig::Ruleset {
                name: "node1".to_owned(),
                rules: vec![new_rule("rule_a1", None), new_rule("rule_a2", op.clone())],
            },
            MatcherConfig::Ruleset {
                name: "node2".to_owned(),
                rules: vec![new_rule("rule_b1", None), new_rule("rule_b2", op)],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(2, nodes.len());

                match nodes.first().unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node1", name);
                        assert_eq!(2, rules.rules.len());

                        assert_eq!(rules.rules.first().unwrap().name, "rule_a1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.first().unwrap().status
                        );

                        assert_eq!(rules.rules.get(1).unwrap().name, "rule_a2");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(1).unwrap().status
                        );
                    }
                    _ => unreachable!(),
                };

                match nodes.get(1).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node2", name);
                        assert_eq!(2, rules.rules.len());

                        assert_eq!(rules.rules.first().unwrap().name, "rule_b1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.first().unwrap().status
                        );

                        assert_eq!(rules.rules.get(1).unwrap().name, "rule_b2");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(1).unwrap().status
                        );
                    }
                    _ => unreachable!(),
                };
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_process_filter_rulesets_recursively() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("${event.type}".to_owned()),
            second: Value::String("email".to_owned()),
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
                filter: new_filter(Operator::Equals {
                    first: Value::String("${event.type}".to_owned()),
                    second: Value::String("trap".to_owned()),
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
                rules: vec![new_rule("rule_b1", None), new_rule("rule_b2", op)],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter1".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!(name, "filter1");
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(4, nodes.len());

                match nodes.first().unwrap() {
                    ProcessedNode::Filter { name, filter, nodes } => {
                        assert_eq!(name, "node0");
                        assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                        assert_eq!(1, nodes.len());

                        match nodes.first().unwrap() {
                            ProcessedNode::Ruleset { name, rules } => {
                                assert_eq!(name, "node");
                                assert_eq!(1, rules.rules.len());
                                assert_eq!(rules.rules.first().unwrap().name, "rule2");
                                assert_eq!(
                                    ProcessedRuleStatus::Matched,
                                    rules.rules.first().unwrap().status
                                );
                            }
                            _ => unreachable!(),
                        };
                    }
                    _ => unreachable!(),
                };

                match nodes.get(1).unwrap() {
                    ProcessedNode::Filter { name, filter, nodes } => {
                        assert_eq!(name, "node1");
                        assert_eq!(ProcessedFilterStatus::NotMatched, filter.status);
                        assert_eq!(0, nodes.len());
                    }
                    _ => unreachable!(),
                };

                match nodes.get(2).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!(name, "node2");
                        assert_eq!(2, rules.rules.len());
                        assert_eq!(rules.rules.first().unwrap().name, "rule_a1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.first().unwrap().status
                        );
                        assert_eq!(rules.rules.get(1).unwrap().name, "rule_a2");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(1).unwrap().status
                        );
                    }
                    _ => unreachable!(),
                };

                match nodes.get(3).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!(name, "node3");
                        assert_eq!(2, rules.rules.len());
                        assert_eq!(rules.rules.first().unwrap().name, "rule_b1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.first().unwrap().status
                        );
                        assert_eq!(rules.rules.get(1).unwrap().name, "rule_b2");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.get(1).unwrap().status
                        );
                    }
                    _ => unreachable!(),
                };
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_process_no_rulesets_if_filter_is_inactive() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("${event.type}".to_owned()),
            second: Value::String("email".to_owned()),
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
                rules: vec![new_rule("rule_b1", op)],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::Inactive, filter.status);
                assert_eq!(0, nodes.len());
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_process_no_rulesets_if_filter_does_not_match() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("${event.type}".to_owned()),
            second: Value::String("email".to_owned()),
        };

        let filter = new_filter(Operator::Equals {
            first: Value::String("${event.type}".to_owned()),
            second: Value::String("trapd".to_owned()),
        });

        let nodes = vec![
            MatcherConfig::Ruleset {
                name: "node0".to_owned(),
                rules: vec![new_rule("rule_a1", op.clone()), new_rule("rule_a2", op.clone())],
            },
            MatcherConfig::Ruleset {
                name: "node1".to_owned(),
                rules: vec![new_rule("rule_b1", op.clone()), new_rule("rule_b2", op)],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::NotMatched, filter.status);
                assert_eq!(0, nodes.len());
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_process_rulesets_independently() {
        // Arrange
        let op = Operator::Equals {
            first: Value::String("${event.type}".to_owned()),
            second: Value::String("email".to_owned()),
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
                rules: vec![rule_c1, new_rule("rule_c2", op)],
            },
        ];

        let config = MatcherConfig::Filter { name: "filter".to_owned(), filter, nodes };

        let matcher = new_matcher(&config).unwrap();

        // Act
        let result = matcher.process(json!(Event::new("email")), false);

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(3, nodes.len());

                match nodes.first().unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node0", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.first().unwrap().name, "rule_a1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.first().unwrap().status
                        );
                    }
                    _ => unreachable!(),
                };

                match nodes.get(1).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node1", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.first().unwrap().name, "rule_b1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.first().unwrap().status
                        );
                    }
                    _ => unreachable!(),
                };

                match nodes.get(2).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node2", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.first().unwrap().name, "rule_c1");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.first().unwrap().status
                        );
                    }
                    _ => unreachable!(),
                };
            }
            _ => unreachable!(),
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
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[a-z]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let mut rule_1 = new_rule("rule", None);

        rule_1.constraint.with.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.payload.value}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
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
        payload.insert("value".to_owned(), Value::String("aaa999".to_owned()));

        // Act
        let result = matcher.process(json!(Event::new_with_payload("email", payload)), false);

        // Assert
        match result.result {
            ProcessedNode::Filter { name, filter, nodes } => {
                assert_eq!("filter", name);
                assert_eq!(ProcessedFilterStatus::Matched, filter.status);
                assert_eq!(2, nodes.len());

                match nodes.first().unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node0", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.first().unwrap().name, "rule");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.first().unwrap().status
                        );
                        assert_eq!(
                            "aaa",
                            rules
                                .extracted_vars
                                .get_from_map("rule")
                                .unwrap()
                                .get_from_map("extracted_temp")
                                .unwrap()
                        );
                    }
                    _ => unreachable!(),
                };

                match nodes.get(1).unwrap() {
                    ProcessedNode::Ruleset { name, rules } => {
                        assert_eq!("node1", name);
                        assert_eq!(1, rules.rules.len());
                        assert_eq!(rules.rules.first().unwrap().name, "rule");
                        assert_eq!(
                            ProcessedRuleStatus::Matched,
                            rules.rules.first().unwrap().status
                        );
                        assert_eq!(
                            "999",
                            rules
                                .extracted_vars
                                .get_from_map("rule")
                                .unwrap()
                                .get_from_map("extracted_temp")
                                .unwrap()
                        );
                    }
                    _ => unreachable!(),
                };
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_match_cmp_operators() {
        // Arrange
        let filename = "./test_resources/v1/rules/004_cmp_operators.json";
        let json = std::fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename));
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
            payload.insert("value".to_owned(), json!(1000));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
            };
        }

        // Value equal to 2000 should not match
        {
            // Act
            payload.insert("value".to_owned(), json!(2000));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(
                        ProcessedRuleStatus::NotMatched,
                        rules.rules.first().unwrap().status
                    );
                }
                _ => unreachable!(),
            };
        }

        // Value less than 0 should match
        {
            // Act
            payload.insert("value".to_owned(), json!(-1000));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
            };
        }

        // Value more than 2000 (not included) should match
        {
            // Act
            payload.insert("value".to_owned(), json!(1000000000.0));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
            };
        }

        // Value between 100 and 200 (included) should match
        {
            // Act
            payload.insert("value".to_owned(), json!(100));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
            };
        }

        // Value between 100 and 200 (included) should match
        {
            // Act
            payload.insert("value".to_owned(), json!(110));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
            };
        }

        // Value between 100 and 200 (included) should match
        {
            // Act
            payload.insert("value".to_owned(), json!(200));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
            };
        }
        // Value equal to 140 should not match
        // test for `NOT` operator
        {
            // Act
            payload.insert("value".to_owned(), json!(140));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(
                        ProcessedRuleStatus::NotMatched,
                        rules.rules.first().unwrap().status
                    );
                }
                _ => unreachable!(),
            };
        }

        // Value equal to 150 should not match
        // test for `ne` operator
        {
            // Act
            payload.insert("value".to_owned(), json!(150));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(
                        ProcessedRuleStatus::NotMatched,
                        rules.rules.first().unwrap().status
                    );
                }
                _ => unreachable!(),
            };
        }

        // Value equal to 160 should not match
        // test for `notEqual` alias
        {
            // Act
            payload.insert("value".to_owned(), json!(160));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(
                        ProcessedRuleStatus::NotMatched,
                        rules.rules.first().unwrap().status
                    );
                }
                _ => unreachable!(),
            };
        }

        // equalsIgnoreCase should match "Warning"
        {
            // Act
            payload.insert(
                "value".to_owned(),
                Value::String("This is a Contain alias test!".to_owned()),
            );
            payload.insert("message".to_owned(), Value::String("WaRnInG".to_owned()));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
            };
        }

        // equalsIgnoreCase should not match "WaRnInGs"
        {
            // Act
            payload.insert(
                "value".to_owned(),
                Value::String("This is a Contain alias test!".to_owned()),
            );
            payload.insert("message".to_owned(), Value::String("WaRnInGs".to_owned()));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(
                        ProcessedRuleStatus::NotMatched,
                        rules.rules.first().unwrap().status
                    );
                }
                _ => unreachable!(),
            };
        }
    }

    #[test]
    fn contains_ignore_case_should_correctly_match() {
        // Arrange
        let filename = "./test_resources/v1/rules/005_contains_operators.json";
        let json = std::fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename));
        let mut rule = Rule::from_json(&json).unwrap();
        rule.name = "ccontains_operators".to_owned();

        let mut payload = Payload::new();
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule.clone()],
        })
        .unwrap();

        // Value containing (case insentitive) "something" should match
        {
            // Act
            payload.insert(
                "value".to_owned(),
                Value::String("The word Something should match".to_owned()),
            );
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
            };
        }
    }

    #[test]
    fn contains_ignore_case_should_correctly_not_match() {
        // Arrange
        let filename = "./test_resources/v1/rules/005_contains_operators.json";
        let json = std::fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename));
        let mut rule = Rule::from_json(&json).unwrap();
        rule.name = "ccontains_operators".to_owned();

        let mut payload = Payload::new();
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule.clone()],
        })
        .unwrap();

        // Value not containing (case insentitive) "something" should not match
        {
            // Act
            payload.insert("value".to_owned(), Value::String("Some".to_owned()));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(
                        ProcessedRuleStatus::NotMatched,
                        rules.rules.first().unwrap().status
                    );
                }
                _ => unreachable!(),
            };
        }
    }

    #[test]
    fn contains_ignore_case_should_correctly_match_with_arrays() {
        // Arrange
        let filename = "./test_resources/v1/rules/005_contains_operators.json";
        let json = std::fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename));
        let mut rule = Rule::from_json(&json).unwrap();
        rule.name = "ccontains_operators".to_owned();

        let mut payload = Payload::new();
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule.clone()],
        })
        .unwrap();

        // Array containing a string equal to (case insentivive) "something" should match
        {
            // Act
            payload.insert(
                "value".to_owned(),
                Value::Array(vec![
                    Value::String("Something else".to_owned()),
                    Value::String("Something".to_owned()),
                ]),
            );
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
            };
        }
    }

    #[test]
    fn contains_ignore_case_should_correctly_not_match_with_arrays() {
        // Arrange
        let filename = "./test_resources/v1/rules/005_contains_operators.json";
        let json = std::fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename));
        let mut rule = Rule::from_json(&json).unwrap();
        rule.name = "ccontains_operators".to_owned();

        let mut payload = Payload::new();
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule.clone()],
        })
        .unwrap();

        // Array not containing a string equal to (case insentivive) "something" should not match
        {
            // Act
            payload.insert(
                "value".to_owned(),
                Value::Array(vec![
                    Value::String("This is Something".to_owned()),
                    Value::String("Something else".to_owned()),
                ]),
            );
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(
                        ProcessedRuleStatus::NotMatched,
                        rules.rules.first().unwrap().status
                    );
                }
                _ => unreachable!(),
            };
        }
    }

    #[test]
    fn contains_should_correctly_match() {
        // Arrange
        let filename = "./test_resources/v1/rules/005_contains_operators.json";
        let json = std::fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename));
        let mut rule = Rule::from_json(&json).unwrap();
        rule.name = "ccontains_operators".to_owned();

        let mut payload = Payload::new();
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule.clone()],
        })
        .unwrap();

        // contains operator should work
        // Value containing "Contains test" should match
        {
            // Act
            payload
                .insert("value".to_owned(), Value::String("This is a Contains test!".to_owned()));
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
            };
        }
    }

    #[test]
    fn contain_alias_should_correctly_match() {
        // Arrange
        let filename = "./test_resources/v1/rules/005_contains_operators.json";
        let json = std::fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to open the file [{}]", filename));
        let mut rule = Rule::from_json(&json).unwrap();
        rule.name = "ccontains_operators".to_owned();

        let mut payload = Payload::new();
        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule.clone()],
        })
        .unwrap();

        // contain alias operator should still work
        // Value containing "Contain alias test" should match
        {
            // Act
            payload.insert(
                "value".to_owned(),
                Value::String("This is a Contain alias test!".to_owned()),
            );
            let result =
                matcher.process(json!(Event::new_with_payload("email", payload.clone())), false);

            // Assert
            match result.result {
                ProcessedNode::Ruleset { name, rules } => {
                    assert_eq!(name, "ruleset");
                    assert_eq!(1, rules.rules.len());
                    assert_eq!(rules.rules.first().unwrap().name, rule.name);
                    assert_eq!(ProcessedRuleStatus::Matched, rules.rules.first().unwrap().status);
                }
                _ => unreachable!(),
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
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[a-z]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let mut rule_2 = new_rule(
            "rule2",
            Operator::Equals {
                first: Value::String("${_variables.rule1.extracted}".to_owned()),
                second: Value::String("aaa".to_owned()),
            },
        );

        rule_2.constraint.with.insert(
            String::from("extracted"),
            Extractor {
                from: String::from("${event.payload.value}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: Some(0),
                    all_matches: false,
                },
                modifiers_post: vec![],
            },
        );

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2],
        })
        .expect("should create a matcher");

        let mut payload = Payload::new();
        payload.insert("value".to_owned(), Value::String("aaa999".to_owned()));

        // Act
        let result = matcher.process(json!(Event::new_with_payload("email", payload)), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(2, rules.rules.len());

                assert_eq!(
                    "aaa",
                    rules
                        .extracted_vars
                        .get_from_map("rule1")
                        .expect("should contain rule1.extracted")
                        .get_from_map("extracted")
                        .expect("should contain rule1.extracted")
                );
                assert_eq!(
                    "999",
                    rules
                        .extracted_vars
                        .get_from_map("rule2")
                        .expect("should contain rule2.extracted")
                        .get_from_map("extracted")
                        .expect("should contain rule1.extracted")
                );

                let rule_1_processed = rules.rules.first().expect("should contain rule1");
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);

                let rule_2_processed = rules.rules.get(1).expect("should contain rule2");
                assert_eq!(ProcessedRuleStatus::Matched, rule_2_processed.status);
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn extracted_variables_name_collisions_should_prioritize_the_local_rule() {
        // Arrange
        let rule_1 = {
            let mut rule = new_rule("collision_name", None);
            rule.constraint.with.insert(
                String::from("VALUE"),
                Extractor {
                    from: String::from("${event.payload.value}"),
                    regex: ExtractorRegex::Regex {
                        regex: String::from(r"[a-z]+"),
                        group_match_idx: Some(0),
                        all_matches: false,
                    },
                    modifiers_post: vec![],
                },
            );

            let mut action = ConfigAction { id: String::from("action_id"), payload: Map::new() };
            action
                .payload
                .insert("value".to_owned(), Value::String("${_variables.VALUE}".to_owned()));
            rule.actions.push(action);
            rule
        };

        let rule_2 = {
            let mut rule = new_rule("rule2", None);
            rule.constraint.with.insert(
                String::from("collision_name"),
                Extractor {
                    from: String::from("${event.payload.value}"),
                    regex: ExtractorRegex::RegexNamedGroups {
                        regex: String::from(r"(?P<VALUE>[0-9]+)"),
                        all_matches: false,
                    },
                    modifiers_post: vec![],
                },
            );

            let mut action = ConfigAction { id: String::from("action_id"), payload: Map::new() };
            action.payload.insert(
                "value".to_owned(),
                Value::String("${_variables.collision_name.VALUE}".to_owned()),
            );
            action.payload.insert(
                "full".to_owned(),
                Value::String("${_variables.collision_name}".to_owned()),
            );
            rule.actions.push(action);
            rule
        };

        let rule_3 = {
            let mut rule = new_rule("rule3", None);

            let mut action = ConfigAction { id: String::from("action_id"), payload: Map::new() };
            action.payload.insert(
                "value".to_owned(),
                Value::String("${_variables.collision_name.VALUE}".to_owned()),
            );
            rule.actions.push(action);
            rule
        };

        let matcher = new_matcher(&MatcherConfig::Ruleset {
            name: "ruleset".to_owned(),
            rules: vec![rule_1, rule_2, rule_3],
        })
        .expect("should create a matcher");

        let mut payload = Payload::new();
        payload.insert("value".to_owned(), Value::String("aaa999".to_owned()));

        // Act
        let result = matcher.process(json!(Event::new_with_payload("email", payload)), false);

        // Assert
        match result.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(3, rules.rules.len());

                assert_eq!(
                    "aaa",
                    rules
                        .extracted_vars
                        .get_from_map("collision_name")
                        .expect("should contain collision_name")
                        .get_from_map("VALUE")
                        .expect("should contain collision_name.VALUE")
                );

                let mut vars = Map::new();
                vars.insert("VALUE".to_owned(), Value::String("999".to_owned()));
                let vars = Value::Object(vars);
                assert_eq!(
                    &vars,
                    rules
                        .extracted_vars
                        .get_from_map("rule2")
                        .expect("should contain rule2")
                        .get_from_map("collision_name")
                        .expect("should contain rule2.collision_name")
                );

                let rule_1_processed = rules.rules.first().expect("should contain rule1");
                assert_eq!(ProcessedRuleStatus::Matched, rule_1_processed.status);

                let rule_2_processed = rules.rules.get(1).expect("should contain rule2");
                assert_eq!(ProcessedRuleStatus::Matched, rule_2_processed.status);
                assert_eq!(&vars, rule_2_processed.actions[0].payload.get("full").unwrap());
                assert_eq!("999", rule_2_processed.actions[0].payload.get("value").unwrap());

                let rule_3_processed = rules.rules.get(2).expect("should contain rule2");
                assert_eq!(ProcessedRuleStatus::Matched, rule_3_processed.status);
                assert_eq!("aaa", rule_3_processed.actions[0].payload.get("value").unwrap());
            }
            _ => unreachable!(),
        };
    }

    #[test]
    fn should_return_processed_rule_metadata() {
        // Arrange
        let mut rule = new_rule("rule_name", None);
        rule.actions.push(ConfigAction { id: String::from("action_1"), payload: Map::new() });
        rule.actions.push(ConfigAction { id: String::from("action_2"), payload: Map::new() });
        rule.actions.push(ConfigAction { id: String::from("action_3"), payload: Map::new() });

        let matcher =
            new_matcher(&MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![rule] })
                .expect("should create a matcher");

        // Act
        let result_without_metadata = matcher.process(json!(Event::new("email")), false);
        let result_with_metadata = matcher.process(json!(Event::new("email")), true);

        // Assert
        match result_without_metadata.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(1, rules.rules.len());

                let rule_processed = rules.rules.first().expect("should contain rule");
                assert!(rule_processed.meta.is_none())
            }
            _ => unreachable!(),
        };

        match result_with_metadata.result {
            ProcessedNode::Ruleset { name, rules } => {
                assert_eq!("ruleset", name);
                assert_eq!(1, rules.rules.len());

                let rule_processed = rules.rules.first().expect("should contain rule");
                assert!(rule_processed.meta.is_some());
                let processed_rule_metadata = rule_processed.meta.as_ref().unwrap();
                assert_eq!(3, processed_rule_metadata.actions.len());
                assert_eq!("action_1", &processed_rule_metadata.actions[0].id);
                assert_eq!("action_2", &processed_rule_metadata.actions[1].id);
                assert_eq!("action_3", &processed_rule_metadata.actions[2].id);
            }
            _ => unreachable!(),
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
        let filter =
            filter.into().map(Defaultable::Value).unwrap_or_else(|| Defaultable::Default {});
        Filter { active: true, description: "".to_owned(), filter }
    }
}
