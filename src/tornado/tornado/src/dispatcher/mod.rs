use actix::prelude::*;
use tornado_engine_matcher::{dispatcher, error, model};

pub mod event_bus;

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
