use actix::prelude::*;
use log::*;
use tornado_common_api::Action;
use tornado_engine_matcher::{dispatcher, error, model};
use tornado_network_common::EventBus;

pub struct ActixEventBus<F: Fn(Action)> {
    pub callback: F,
}

impl<F: Fn(Action)> EventBus for ActixEventBus<F> {
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
    dispatcher: dispatcher::Dispatcher,
}

impl DispatcherActor {
    pub fn start_new(
        message_mailbox_capacity: usize,
        dispatcher: dispatcher::Dispatcher,
    ) -> Addr<Self> {
        Self::create(move |ctx| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            Self { dispatcher }
        })
    }
}

impl Actor for DispatcherActor {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("DispatcherActor started.");
    }
}

impl Handler<ProcessedEventMessage> for DispatcherActor {
    type Result = Result<(), error::MatcherError>;

    fn handle(&mut self, msg: ProcessedEventMessage, _: &mut Context<Self>) -> Self::Result {
        trace!("DispatcherActor - received new processed event [{:?}]", &msg.event);
        self.dispatcher.dispatch_actions(msg.event.result)
    }
}
