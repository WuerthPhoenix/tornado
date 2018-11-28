use actix::prelude::*;
use actors::uds_writer::{EventMessage, UdsWriterActor};
use tornado_collector_common::Collector;
use tornado_collector_rsyslog::RsyslogCollector;

pub struct RsyslogCollectorActor {
    pub collector: RsyslogCollector,
    pub writer_addr: Addr<UdsWriterActor>,
}

#[derive(Message)]
pub struct RsyslogMessage(pub String);

impl RsyslogCollectorActor {
    pub fn new(writer_addr: Addr<UdsWriterActor>) -> RsyslogCollectorActor {
        RsyslogCollectorActor { collector: RsyslogCollector::new(), writer_addr }
    }
}

impl Actor for RsyslogCollectorActor {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("RsyslogCollectorActor started.");
    }
}

impl Handler<RsyslogMessage> for RsyslogCollectorActor {
    type Result = ();

    fn handle(&mut self, msg: RsyslogMessage, _: &mut SyncContext<Self>) -> Self::Result {
        warn!("JsonReaderActor - received msg: [{}]", &msg.0);

        match self.collector.to_event(&msg.0) {
            Ok(event) => self.writer_addr.do_send(EventMessage { event }),
            Err(e) => error!("JsonReaderActor - Cannot unmarshal event from json: {}", e),
        };
    }
}