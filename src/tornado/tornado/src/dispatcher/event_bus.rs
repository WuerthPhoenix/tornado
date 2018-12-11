use tornado_network_common::EventBus;
use tornado_common_api::Action;

pub struct ActixEventBus {}

impl EventBus for ActixEventBus {
    fn publish_action(&self, message: Action) {

        match message.id.as_ref() {
            "archive" => {
                warn!("Archive action received")
            },
            _ => warn!("No executors registered for action with id [{}]", message.id)
        }

    }
}