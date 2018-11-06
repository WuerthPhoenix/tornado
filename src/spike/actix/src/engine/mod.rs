use actix::prelude::*;
use executor::{ExecutorActor, ProcessedEventMessage};
use std::sync::Arc;
use std::thread;
use tornado_common_api;
use tornado_engine_matcher::{error, matcher};

pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

impl Message for EventMessage {
    type Result = Result<(), error::MatcherError>;
}

pub struct MatcherActor {
    pub executor_addr: Addr<ExecutorActor>,
    pub matcher: Arc<matcher::Matcher>,
}

impl Actor for MatcherActor {
    type Context = SyncContext<Self>;
}

impl Handler<EventMessage> for MatcherActor {
    type Result = Result<(), error::MatcherError>;

    fn handle(&mut self, msg: EventMessage, _: &mut SyncContext<Self>) -> Self::Result {
        debug!("MatcherActor - {:?} - received new event", thread::current().name());
        let processed_event = self.matcher.process(msg.event);
        self.executor_addr.do_send(ProcessedEventMessage { event: processed_event });
        Ok(())
    }
}
