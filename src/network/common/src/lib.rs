use tornado_common_api::Action;

pub trait EventBus {
    fn publish_action(&self, message: Action);
}
