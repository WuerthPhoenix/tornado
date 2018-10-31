extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_engine_matcher as matcher;
extern crate tornado_network_common;
extern crate tornado_network_simple;

extern crate actix;
extern crate futures;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_uds;

use actix::prelude::*;
use futures::Future;

struct EventMessage {
    event: tornado_common_api::Event
}

impl Message for EventMessage {
    type Result = Result<(), matcher::error::MatcherError>;
}

struct MatcherActor {
    matcher: matcher::matcher::Matcher,
    dispatcher: matcher::dispatcher::Dispatcher
}

impl Actor for MatcherActor {
    type Context = Context<Self>;
}

impl Handler<EventMessage> for MatcherActor {
    type Result = Result<(), matcher::error::MatcherError>;

    fn handle(&mut self, msg: EventMessage, _: &mut Context<Self>) -> Self::Result {
        let processed_event = self.matcher.process(msg.event);
        self.dispatcher.dispatch_actions(&processed_event)
    }
}

#[cfg(test)]
extern crate tempfile;