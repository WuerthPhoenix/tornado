extern crate tornado_common_api;

use tornado_common_api::Action;

/// Temporary dumb implementation.
/// To redo when the network layer will be better defined.
pub trait EventBus {
    fn publish_action(&self, message: &Action);

    fn subscribe_to_action(
        &mut self,
        action_id: &str,
        handler: Box<'static + Fn(&Action) -> () + Sync + Send>,
    );
}
