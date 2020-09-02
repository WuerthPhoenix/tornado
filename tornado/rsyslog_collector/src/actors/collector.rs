use actix::dev::ToEnvelope;
use actix::prelude::*;
use log::*;
use tornado_collector_common::Collector;
use tornado_collector_json::JsonPayloadCollector;
use tornado_common::actors::message::{EventMessage, StringMessage};

pub struct RsyslogCollectorActor<A: Actor + actix::Handler<EventMessage>>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    collector: JsonPayloadCollector,
    writer_addr: Addr<A>,
}

impl<A: Actor + actix::Handler<EventMessage>> RsyslogCollectorActor<A>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    pub fn start_new(writer_addr: Addr<A>, message_queue_size: usize) -> Addr<Self> {
        RsyslogCollectorActor::create(move |ctx| {
            ctx.set_mailbox_capacity(message_queue_size);
            RsyslogCollectorActor { collector: JsonPayloadCollector::new("syslog"), writer_addr }
        })
    }
}

impl<A: Actor + actix::Handler<EventMessage>> Actor for RsyslogCollectorActor<A>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("RsyslogCollectorActor started.");
    }
}

impl<A: Actor + actix::Handler<EventMessage>> Handler<StringMessage> for RsyslogCollectorActor<A>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    type Result = ();

    fn handle(&mut self, msg: StringMessage, _: &mut Context<Self>) -> Self::Result {
        debug!("RsyslogCollectorActor - received msg: [{}]", &msg.msg);

        match self.collector.to_event(&msg.msg) {
            Ok(event) => self.writer_addr.try_send(EventMessage { event }).unwrap_or_else(|err| {
                error!("RsyslogCollectorActor - Error while sending event. Error: {}", err)
            }),
            Err(e) => error!("RsyslogCollectorActor - Cannot unmarshal event from json: {}", e),
        };
    }
}
