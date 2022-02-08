use tornado_common::actors::message::ActionMessage;

pub trait EventBus {
    fn publish_action(&self, message: ActionMessage);
}
