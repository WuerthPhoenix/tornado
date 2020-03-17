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
    pub collector: JsonPayloadCollector,
    pub writer_addr: Addr<A>,
}

impl<A: Actor + actix::Handler<EventMessage>> RsyslogCollectorActor<A>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    pub fn new(writer_addr: Addr<A>) -> Self {
        RsyslogCollectorActor { collector: JsonPayloadCollector::new("syslog"), writer_addr }
    }
}

impl<A: Actor + actix::Handler<EventMessage>> Actor for RsyslogCollectorActor<A>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("RsyslogCollectorActor started.");
    }
}

impl<A: Actor + actix::Handler<EventMessage>> Handler<StringMessage> for RsyslogCollectorActor<A>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    type Result = ();

    fn handle(&mut self, msg: StringMessage, _: &mut SyncContext<Self>) -> Self::Result {
        debug!("RsyslogCollectorActor - received msg: [{}]", &msg.msg);

        match self.collector.to_event(&msg.msg) {
            Ok(event) => self.writer_addr.do_send(EventMessage { event }),
            Err(e) => error!("RsyslogCollectorActor - Cannot unmarshal event from json: {}", e),
        };
    }
}
