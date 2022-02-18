use std::collections::HashMap;
use tornado_common::actors::message::ActionMessage;
use tornado_network_common::EventBus;

#[derive(Default)]
pub struct SimpleEventBus {
    subscribers: HashMap<String, Box<dyn 'static + Fn(ActionMessage) + Sync + Send>>,
}

impl SimpleEventBus {
    pub fn new() -> SimpleEventBus {
        SimpleEventBus { subscribers: HashMap::new() }
    }
}

impl SimpleEventBus {
    pub fn subscribe_to_action(
        &mut self,
        action_id: &str,
        handler: Box<dyn 'static + Fn(ActionMessage) + Sync + Send>,
    ) {
        self.subscribers.insert(action_id.to_owned(), handler);
    }
}

impl EventBus for SimpleEventBus {
    fn publish_action(&self, message: ActionMessage) {
        if let Some(handler) = self.subscribers.get(&message.0.action.id) {
            handler(message)
        };
    }
}

#[cfg(test)]
mod test {
    use tornado_common_api::{Action, Map, TracedAction};

    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::Span;

    #[test]
    fn should_subscribe_and_be_called() {
        // Arrange
        let mut bus = SimpleEventBus::new();
        let action_id = "test";
        let received = Arc::new(Mutex::new(String::from("")));

        let clone = received.clone();
        bus.subscribe_to_action(
            action_id,
            Box::new(move |message: ActionMessage| {
                println!("received action of id: {}", message.0.action.id);
                let mut value = clone.lock().unwrap();
                *value = message.0.action.id.clone();
            }),
        );

        let action = ActionMessage(TracedAction {
            span: Span::current(),
            action: Arc::new(Action { id: String::from(action_id), payload: Map::new() }),
        });

        // Act
        bus.publish_action(action);

        // Assert
        let value = &*received.lock().unwrap();
        assert_eq!(action_id, value)
    }
}
