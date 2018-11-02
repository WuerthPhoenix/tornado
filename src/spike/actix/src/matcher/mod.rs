use actix::prelude::*;
use futures::Future;
use std::sync::Arc;
use tornado_common_api;
use tornado_common_logger;
use tornado_engine_matcher::{matcher, error, dispatcher};
use tornado_network_common;
use tornado_network_simple;

pub struct EventMessage {
    pub event: tornado_common_api::Event
}

impl Message for EventMessage {
    type Result = Result<(), error::MatcherError>;
}

pub struct MatcherActor {
    pub matcher: Arc<matcher::Matcher>,
    pub dispatcher: Arc<dispatcher::Dispatcher>
}

impl Actor for MatcherActor {
    type Context = Context<Self>;
}

impl Handler<EventMessage> for MatcherActor {
    type Result = Result<(), error::MatcherError>;

    fn handle(&mut self, msg: EventMessage, _: &mut Context<Self>) -> Self::Result {
        let processed_event = self.matcher.process(msg.event);
        self.dispatcher.dispatch_actions(&processed_event)
    }
}