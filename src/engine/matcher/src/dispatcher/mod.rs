use error::MatcherError;
use model::{ProcessedEvent, ProcessedRuleStatus};
use std::sync::Arc;
use tornado_common_api::Action;
use tornado_network_common::EventBus;

/// The dispatcher is in charge of dispatching the Actions defined in a ProcessedEvent.
// ToDo: the current implementation is temporary. This is going to change deeply when the network layer is defined.
pub struct Dispatcher {
    event_bus: Arc<EventBus>,
}

impl Dispatcher {
    pub fn new(event_bus: Arc<EventBus>) -> Result<Dispatcher, MatcherError> {
        Ok(Dispatcher { event_bus })
    }

    /// Receives a fully processed ProcessedEvent and dispatches the actions linked to Rules whose status is Matched.
    /// The actions resolution (i.e. resolving the extracted variables, filling the action payload, etc.) is supposed to be performed before this method execution.
    pub fn dispatch_actions(&self, event: &ProcessedEvent) -> Result<(), MatcherError> {
        for (rule_name, rule) in &event.rules {
            match rule.status {
                ProcessedRuleStatus::Matched => self.dispatch(&rule.actions)?,
                _ => {
                    trace!("Rule [{}] not matched, ignoring actions", rule_name);
                }
            }
        }
        Ok(())
    }

    fn dispatch(&self, actions: &[Action]) -> Result<(), MatcherError> {
        for action in actions {
            // ToDo: avoid cloning. To be fixed when implementing the underlying network as the object could be serialized here.
            self.event_bus.publish_action(action.clone())
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use model::ProcessedRule;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tornado_common_api::Event;
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
                Box::new(move |message: Action| {
                    println!("received action of id: {}", message.id);
                    let mut value = clone.lock().unwrap();
                    value.push(message)
                }),
            );
        }

        let dispatcher = Dispatcher::new(Arc::new(bus)).unwrap();

        let mut rule = ProcessedRule::new("rule1".to_owned());
        rule.status = ProcessedRuleStatus::Matched;
        rule.actions.push(Action { id: action_id.clone(), payload: HashMap::new() });
        rule.actions.push(Action { id: action_id.clone(), payload: HashMap::new() });

        let mut event = ProcessedEvent::new(Event::new("".to_owned()));
        event.rules.insert("rule1".to_owned(), rule);

        // Act
        dispatcher.dispatch_actions(&event).unwrap();

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
                Box::new(move |message: Action| {
                    println!("received action of id: {}", message.id);
                    let mut value = clone.lock().unwrap();
                    value.push(message)
                }),
            );
        }

        let dispatcher = Dispatcher::new(Arc::new(bus)).unwrap();

        let mut rule = ProcessedRule::new("rule1".to_owned());
        rule.actions.push(Action { id: action_id.clone(), payload: HashMap::new() });

        let mut event = ProcessedEvent::new(Event::new("".to_owned()));
        event.rules.insert("rule1".to_owned(), rule);

        // Act
        dispatcher.dispatch_actions(&event).unwrap();

        // Assert
        assert_eq!(0, received.lock().unwrap().len());
    }

}
