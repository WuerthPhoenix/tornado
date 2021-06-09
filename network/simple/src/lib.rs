use std::collections::HashMap;
use tornado_common_api::Action;
use tornado_network_common::EventBus;

#[derive(Default)]
pub struct SimpleEventBus {
    subscribers: HashMap<String, Box<dyn 'static + Fn(Action) + Sync + Send>>,
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
        handler: Box<dyn 'static + Fn(Action) + Sync + Send>,
    ) {
        self.subscribers.insert(action_id.to_owned(), handler);
    }
}

impl EventBus for SimpleEventBus {
    fn publish_action(&self, message: Action) {
        if let Some(handler) = self.subscribers.get(&message.id) {
            handler(message)
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn should_subscribe_and_be_called() {
        // Arrange
        let mut bus = SimpleEventBus::new();
        let action_id = "test";
        let received = Arc::new(Mutex::new(String::from("")));

        let clone = received.clone();
        bus.subscribe_to_action(
            action_id,
            Box::new(move |message: Action| {
                println!("received action of id: {}", message.id);
                let mut value = clone.lock().unwrap();
                *value = message.id.clone();
            }),
        );

        // Act
        bus.publish_action(Action { trace_id: "".to_owned(), id: String::from(action_id), payload: HashMap::new() });

        // Assert
        let value = &*received.lock().unwrap();
        assert_eq!(action_id, value)
    }
}
