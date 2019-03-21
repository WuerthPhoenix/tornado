use actix::prelude::*;
use log::*;
use tornado_collector_common::Collector;
use tornado_collector_json::JsonPayloadCollector;
use tornado_common::actors::uds_client::{EventMessage, UdsClientActor};

pub struct RsyslogCollectorActor {
    pub collector: JsonPayloadCollector,
    pub writer_addr: Addr<UdsClientActor>,
}

#[derive(Message)]
pub struct RsyslogMessage {
    pub json: String,
}

impl RsyslogCollectorActor {
    pub fn new(writer_addr: Addr<UdsClientActor>) -> RsyslogCollectorActor {
        RsyslogCollectorActor { collector: JsonPayloadCollector::new("syslog"), writer_addr }
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
        debug!("JsonReaderActor - received msg: [{}]", &msg.json);

        match self.collector.to_event(&msg.json) {
            Ok(event) => self.writer_addr.do_send(EventMessage { event }),
            Err(e) => error!("JsonReaderActor - Cannot unmarshal event from json: {}", e),
        };
    }
}
