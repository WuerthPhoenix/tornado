use actix::prelude::*;
use dispatcher::{DispatcherActor, ProcessedEventMessage};
use std::sync::Arc;
use tornado_common_api;
use tornado_engine_matcher::{error, matcher};

pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

impl Message for EventMessage {
    type Result = Result<(), error::MatcherError>;
}

pub struct MatcherActor {
    pub dispatcher_addr: Addr<DispatcherActor>,
    pub matcher: Arc<matcher::Matcher>,
}

impl Actor for MatcherActor {
    type Context = SyncContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("MatcherActor started.");
    }
}

impl Handler<EventMessage> for MatcherActor {
    type Result = Result<(), error::MatcherError>;

    fn handle(&mut self, msg: EventMessage, _: &mut SyncContext<Self>) -> Self::Result {
        debug!("MatcherActor - received new event [{:?}]", &msg.event);
        let processed_event = self.matcher.process(msg.event);
        self.dispatcher_addr.do_send(ProcessedEventMessage { event: processed_event });
        Ok(())
    }
}
