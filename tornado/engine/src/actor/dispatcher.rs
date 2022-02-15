use actix::prelude::*;
use log::*;
use tornado_common::actors::message::ActionMessage;
use tornado_engine_matcher::{dispatcher, error, model};
use tornado_network_common::EventBus;
use tracing::Span;

pub struct ActixEventBus<F: Fn(ActionMessage)> {
    pub callback: F,
}

impl<F: Fn(ActionMessage)> EventBus for ActixEventBus<F> {
    fn publish_action(&self, message: ActionMessage) {
        (self.callback)(message)
    }
}

pub struct ProcessedEventMessage {
    pub span: Span,
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
        let _span = msg.span.entered();
        let _emit_matched_action_span = tracing::debug_span!("Emit matched Actions").entered();

        trace!("DispatcherActor - received new processed event [{:?}]", &msg.event);
        self.dispatcher.dispatch_actions(msg.event.result)
    }
}
