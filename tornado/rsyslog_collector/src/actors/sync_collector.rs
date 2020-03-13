use actix::prelude::*;
use log::*;
use tornado_collector_common::Collector;
use tornado_collector_json::JsonPayloadCollector;
use tornado_common::actors::message::{EventMessage, StringMessage};
use tornado_common::actors::tcp_client::TcpClientActor;

pub struct RsyslogCollectorActor {
    pub collector: JsonPayloadCollector,
    pub writer_addr: Addr<TcpClientActor>,
}

impl RsyslogCollectorActor {
    pub fn new(writer_addr: Addr<TcpClientActor>) -> RsyslogCollectorActor {
        RsyslogCollectorActor { collector: JsonPayloadCollector::new("syslog"), writer_addr }
    }
}

impl Actor for RsyslogCollectorActor {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("RsyslogCollectorActor started.");
    }
}

impl Handler<StringMessage> for RsyslogCollectorActor {
    type Result = ();

    fn handle(&mut self, msg: StringMessage, _: &mut SyncContext<Self>) -> Self::Result {
        debug!("RsyslogCollectorActor - received msg: [{}]", &msg.msg);

        match self.collector.to_event(&msg.msg) {
            Ok(event) => self.writer_addr.do_send(EventMessage { event }),
            Err(e) => error!("RsyslogCollectorActor - Cannot unmarshal event from json: {}", e),
        };
    }
}
