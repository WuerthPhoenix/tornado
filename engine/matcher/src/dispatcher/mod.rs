use crate::error::MatcherError;
use crate::model::{ProcessedNode, ProcessedRuleStatus};
use log::*;
use std::sync::Arc;
use tornado_common::actors::message::ActionMessage;
use tornado_common_api::{Action, TracedAction};
use tornado_network_common::EventBus;

/// The dispatcher is in charge of dispatching the Actions defined in a ProcessedEvent.
pub struct Dispatcher {
    event_bus: Arc<dyn EventBus>,
}

impl Dispatcher {
    pub fn build(event_bus: Arc<dyn EventBus>) -> Result<Dispatcher, MatcherError> {
        Ok(Dispatcher { event_bus })
    }

    /// Receives a fully processed ProcessedNode and dispatches the actions linked to Rules whose status is Matched.
    /// The action's resolution (i.e. resolving the extracted variables, filling the action payload, etc.) should be completed before this method is executed.
    pub fn dispatch_actions(&self, processed_node: ProcessedNode) -> Result<(), MatcherError> {
        match processed_node {
            ProcessedNode::Ruleset { rules, name, .. } => {
                let _span = tracing::error_span!(
                    "dispatch_ruleset",
                    name = name.as_str(),
                    otel.name = format!("Emit Actions of Ruleset: {}", name).as_str()
                )
                .entered();
                for rule in rules.rules {
                    let _span = tracing::error_span!(
                        "dispatch_rule",
                        name = rule.name.as_str(),
                        otel.name = format!("Emit Actions of Rule: {}", rule.name).as_str()
                    )
                    .entered();
                    match rule.status {
                        ProcessedRuleStatus::Matched => {
                            debug!("Rule [{}] matched, dispatching actions", rule.name);
                            self.dispatch(rule.actions)?
                        }
                        _ => {
                            trace!("Rule [{}] not matched, ignoring actions", rule.name);
                        }
                    }
                }
            }
            ProcessedNode::Filter { nodes, name, .. } => {
                let _span = tracing::error_span!(
                    "dispatch_filter",
                    name = name.as_str(),
                    otel.name = format!("Emit Actions of Filter: {}", name).as_str()
                )
                .entered();
                for node in nodes {
                    self.dispatch_actions(node)?;
                }
            }
            ProcessedNode::Iterator { name, events, .. } => {
                let _span = tracing::error_span!(
                    "dispatch_iterator",
                    name = name.as_str(),
                    otel.name = format!("Emit Actions of Iterator: {}", name).as_str()
                )
                .entered();
                for event in events {
                    for node in event.result {
                        self.dispatch_actions(node)?;
                    }
                }
            }
        };
        Ok(())
    }

    fn dispatch(&self, actions: Vec<Action>) -> Result<(), MatcherError> {
        for (index, action) in actions.into_iter().enumerate() {
            let _span = tracing::error_span!(
                "dispatch_action",
                action = index,
                action_id = action.id.as_str(),
                otel.name = format!("Emit Action: {}", &action.id).as_str(),
            )
            .entered();
            let action_message = ActionMessage(TracedAction {
                span: tracing::Span::current(),
                action: Arc::new(action),
            });

            self.event_bus.publish_action(action_message)
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::{ProcessedFilter, ProcessedFilterStatus, ProcessedRule, ProcessedRules};
    use std::sync::{Arc, Mutex};
    use tornado_common_api::{Action, Map, Value};
    use tornado_network_simple::SimpleEventBus;

    #[test]
    fn should_publish_all_actions() {
        // Arrange
        let mut bus = SimpleEventBus::new();
        let received = Arc::new(Mutex::new(vec![]));

        let action_id = String::from("action1");

        {
            let clone = received.clone();
            bus.subscribe_to_action(
                "action1",
                Box::new(move |message: ActionMessage| {
                    println!("received action of id: {}", message.0.action.id);
                    let mut value = clone.lock().unwrap();
                    value.push(message.0.action)
                }),
            );
        }

        let dispatcher = Dispatcher::build(Arc::new(bus)).unwrap();

        let mut rule = ProcessedRule::new("rule1".to_owned());
        rule.status = ProcessedRuleStatus::Matched;
        rule.actions.push(Action::new(action_id.clone()));
        rule.actions.push(Action::new(action_id));

        let node = ProcessedNode::Ruleset {
            name: "".to_owned(),
            rules: ProcessedRules { rules: vec![rule], extracted_vars: Value::Object(Map::new()) },
        };

        // Act
        dispatcher.dispatch_actions(node).unwrap();

        // Assert
        assert_eq!(2, received.lock().unwrap().len());
    }

    #[test]
    fn should_not_publish_if_rule_not_matched() {
        // Arrange
        let mut bus = SimpleEventBus::new();
        let received = Arc::new(Mutex::new(vec![]));

        let action_id = String::from("action1");

        {
            let clone = received.clone();
            bus.subscribe_to_action(
                "action1",
                Box::new(move |message: ActionMessage| {
                    println!("received action of id: {}", message.0.action.id);
                    let mut value = clone.lock().unwrap();
                    value.push(message.0.action)
                }),
            );
        }

        let dispatcher = Dispatcher::build(Arc::new(bus)).unwrap();

        let mut rule = ProcessedRule::new("rule1".to_owned());
        rule.actions.push(Action::new(action_id));

        let node = ProcessedNode::Ruleset {
            name: "".to_owned(),
            rules: ProcessedRules { rules: vec![rule], extracted_vars: Value::Object(Map::new()) },
        };

        // Act
        dispatcher.dispatch_actions(node).unwrap();

        // Assert
        assert_eq!(0, received.lock().unwrap().len());
    }

    #[test]
    fn should_publish_actions_recursively() {
        // Arrange
        let mut bus = SimpleEventBus::new();
        let received = Arc::new(Mutex::new(vec![]));

        let action_id = String::from("action1");

        {
            let clone = received.clone();
            bus.subscribe_to_action(
                "action1",
                Box::new(move |message: ActionMessage| {
                    println!("received action of id: {}", message.0.action.id);
                    let mut value = clone.lock().unwrap();
                    value.push(message.0.action)
                }),
            );
        }

        let dispatcher = Dispatcher::build(Arc::new(bus)).unwrap();

        let mut rule = ProcessedRule::new("rule1".to_owned());
        rule.status = ProcessedRuleStatus::Matched;
        rule.actions.push(Action::new(action_id));

        let node = ProcessedNode::Filter {
            name: "".to_owned(),
            filter: ProcessedFilter { status: ProcessedFilterStatus::Matched },
            nodes: vec![
                ProcessedNode::Ruleset {
                    name: "node0".to_owned(),
                    rules: ProcessedRules {
                        rules: vec![rule.clone()],
                        extracted_vars: Value::Object(Map::new()),
                    },
                },
                ProcessedNode::Ruleset {
                    name: "node1".to_owned(),
                    rules: ProcessedRules {
                        rules: vec![rule.clone()],
                        extracted_vars: Value::Object(Map::new()),
                    },
                },
            ],
        };

        // Act
        dispatcher.dispatch_actions(node).unwrap();

        // Assert
        assert_eq!(2, received.lock().unwrap().len());
    }
}
