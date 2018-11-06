use actix::prelude::*;
use std::thread;
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
}

impl Handler<ProcessedEventMessage> for ExecutorActor {
    type Result = Result<(), error::MatcherError>;

    fn handle(&mut self, msg: ProcessedEventMessage, _: &mut SyncContext<Self>) -> Self::Result {
        debug!("ExecutorActor - {:?} - received new processed event", thread::current().name());
        self.dispatcher.dispatch_actions(&msg.event)
    }
}
