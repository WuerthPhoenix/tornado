use crate::dispatcher::{DispatcherActor, ProcessedEventMessage};
use actix::prelude::*;
use log::*;
use std::sync::Arc;
use tornado_common_api::Event;
use tornado_engine_api::event::api::ProcessType;
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigReader};
use tornado_engine_matcher::error::MatcherError;
use tornado_engine_matcher::matcher::Matcher;
use tornado_engine_matcher::model::ProcessedEvent;
use tornado_engine_matcher::{error, matcher};

#[derive(Message)]
#[rtype(result = "Result<ProcessedEvent, error::MatcherError>")]
pub struct EventMessageWithReply {
    pub event: tornado_common_api::Event,
    pub process_type: ProcessType,
}

#[derive(Debug, Message)]
#[rtype(result = "Result<ProcessedEvent, error::MatcherError>")]
pub struct EventMessageAndConfigWithReply {
    pub event: tornado_common_api::Event,
    pub matcher_config: MatcherConfig,
    pub process_type: ProcessType,
}

#[derive(Message)]
#[rtype(result = "Result<(), error::MatcherError>")]
pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

#[derive(Message)]
#[rtype(result = "Result<Arc<MatcherConfig>, error::MatcherError>")]
pub struct ReconfigureMessage {}

#[derive(Message)]
#[rtype(result = "Arc<MatcherConfig>")]
pub struct GetCurrentConfigMessage {}

pub struct MatcherActor {
    dispatcher_addr: Addr<DispatcherActor>,
    matcher_config_manager: Arc<dyn MatcherConfigReader>,
    matcher_config: Arc<MatcherConfig>,
    matcher: Arc<matcher::Matcher>,
}

impl MatcherActor {
    pub fn start(
        dispatcher_addr: Addr<DispatcherActor>,
        matcher_config_manager: Arc<dyn MatcherConfigReader>,
        message_mailbox_capacity: usize,
    ) -> Result<Addr<MatcherActor>, MatcherError> {
        let matcher_config = Arc::new(matcher_config_manager.get_config()?);
        let matcher = Arc::new(Matcher::build(&matcher_config)?);
        Ok(actix::Supervisor::start(move |ctx: &mut Context<MatcherActor>| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            MatcherActor { dispatcher_addr, matcher_config_manager, matcher_config, matcher }
        }))
    }

    fn process_event_with_reply(
        &self,
        matcher: &Matcher,
        event: Event,
        process_type: ProcessType,
    ) -> Result<ProcessedEvent, error::MatcherError> {
        let processed_event = matcher.process(event);

        match process_type {
            ProcessType::Full => self
                .dispatcher_addr
                .try_send(ProcessedEventMessage { event: processed_event.clone() }).unwrap_or_else(|err| error!("MatcherActor -  Error while sending ProcessedEventMessage to DispatcherActor. Error: {}", err)),
            ProcessType::SkipActions => {}
        }

        Ok(processed_event)
    }
}

impl Actor for MatcherActor {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("MatcherActor started.");
    }
}

impl actix::Supervised for MatcherActor {
    fn restarting(&mut self, _ctx: &mut Context<MatcherActor>) {
        debug!("MatcherActor restarted.");
    }
}

impl Handler<EventMessage> for MatcherActor {
    type Result = Result<(), error::MatcherError>;

    fn handle(&mut self, msg: EventMessage, _: &mut Context<Self>) -> Self::Result {
        trace!("MatcherActor - received new EventMessage [{:?}]", &msg.event);

        let matcher = self.matcher.clone();
        let dispatcher_addr = self.dispatcher_addr.clone();
        actix::spawn(async move {
            let processed_event = matcher.process(msg.event);
            dispatcher_addr.try_send(ProcessedEventMessage { event: processed_event }).unwrap_or_else(|err| error!("MatcherActor -  Error while sending ProcessedEventMessage to DispatcherActor. Error: {}", err));
        });
        Ok(())
    }
}

impl Handler<EventMessageWithReply> for MatcherActor {
    type Result = Result<ProcessedEvent, error::MatcherError>;

    fn handle(&mut self, msg: EventMessageWithReply, _: &mut Context<Self>) -> Self::Result {
        trace!("MatcherActor - received new EventMessageWithReply [{:?}]", &msg.event);
        self.process_event_with_reply(&self.matcher, msg.event, msg.process_type)
    }
}

impl Handler<EventMessageAndConfigWithReply> for MatcherActor {
    type Result = Result<ProcessedEvent, error::MatcherError>;

    fn handle(
        &mut self,
        msg: EventMessageAndConfigWithReply,
        _: &mut Context<Self>,
    ) -> Self::Result {
        trace!("MatcherActor - received new EventMessageAndConfigWithReply [{:?}]", msg);
        let matcher = Matcher::build(&msg.matcher_config)?;
        self.process_event_with_reply(&matcher, msg.event, msg.process_type)
    }
}

impl Handler<GetCurrentConfigMessage> for MatcherActor {
    type Result = Arc<MatcherConfig>;

    fn handle(&mut self, _msg: GetCurrentConfigMessage, _: &mut Context<Self>) -> Self::Result {
        trace!("MatcherActor - received new GetCurrentConfigMessage");
        self.matcher_config.clone()
    }
}

impl Handler<ReconfigureMessage> for MatcherActor {
    type Result = Result<Arc<MatcherConfig>, error::MatcherError>;

    fn handle(&mut self, _msg: ReconfigureMessage, _: &mut Context<Self>) -> Self::Result {
        info!("MatcherActor - received ReconfigureMessage.");

        let matcher_config = Arc::new(self.matcher_config_manager.get_config()?);
        let matcher = Arc::new(Matcher::build(&matcher_config)?);
        self.matcher_config = matcher_config.clone();
        self.matcher = matcher;

        info!("MatcherActor - Tornado configuration updated successfully.");

        Ok(matcher_config)
    }
}
