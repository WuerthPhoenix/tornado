use actix::prelude::*;
use std::sync::Arc;
use std::thread;
use tornado_common_api;
use tornado_engine_matcher::{dispatcher, error, matcher};

pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

impl Message for EventMessage {
    type Result = Result<(), error::MatcherError>;
}

pub struct MatcherActor {
    pub matcher: Arc<matcher::Matcher>,
    pub dispatcher: dispatcher::Dispatcher,
}

impl Actor for MatcherActor {
    type Context = SyncContext<Self>;
}

impl Handler<EventMessage> for MatcherActor {
    type Result = Result<(), error::MatcherError>;

    fn handle(&mut self, msg: EventMessage, _: &mut SyncContext<Self>) -> Self::Result {
        info!("MatcherActor - {:?} - received new event", thread::current().name());

        let processed_event = self.matcher.process(msg.event);
        self.dispatcher.dispatch_actions(&processed_event)
    }
}
