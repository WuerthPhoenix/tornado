use actix::prelude::*;
use tornado_common_api::Action;
use tornado_engine_matcher::{dispatcher, error, model};
use tornado_network_common::EventBus;

pub struct ActixEventBus<F: Fn(Action) -> ()> {
    pub callback: F,
}

impl<F: Fn(Action) -> ()> EventBus for ActixEventBus<F> {
    fn publish_action(&self, message: Action) {
        (self.callback)(message)
    }
}

pub struct ProcessedEventMessage {
    pub event: model::ProcessedEvent,
}

impl Message for ProcessedEventMessage {
    type Result = Result<(), error::MatcherError>;
}

pub struct DispatcherActor {
    pub dispatcher: dispatcher::Dispatcher,
}

impl Actor for DispatcherActor {
    type Context = SyncContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("DispatcherActor started.");
    }
}

impl Handler<ProcessedEventMessage> for DispatcherActor {
    type Result = Result<(), error::MatcherError>;

    fn handle(&mut self, msg: ProcessedEventMessage, _: &mut SyncContext<Self>) -> Self::Result {
        debug!("DispatcherActor - received new processed event [{:?}]", &msg.event);
        self.dispatcher.dispatch_actions(msg.event)
    }
}
