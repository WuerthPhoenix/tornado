use actix::prelude::*;
use tornado_engine_matcher::{dispatcher, error, model};

pub struct ProcessedEventMessage {
    pub event: model::ProcessedEvent,
}

impl Message for ProcessedEventMessage {
    type Result = Result<(), error::MatcherError>;
}

pub struct ExecutorActor {
    pub dispatcher: dispatcher::Dispatcher,
}

impl Actor for ExecutorActor {
    type Context = SyncContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("ExecutorActor started.");
    }
}

impl Handler<ProcessedEventMessage> for ExecutorActor {
    type Result = Result<(), error::MatcherError>;

    fn handle(&mut self, msg: ProcessedEventMessage, _: &mut SyncContext<Self>) -> Self::Result {
        debug!("ExecutorActor - received new processed event [{:?}]", &msg.event);
        self.dispatcher.dispatch_actions(&msg.event)
    }
}
