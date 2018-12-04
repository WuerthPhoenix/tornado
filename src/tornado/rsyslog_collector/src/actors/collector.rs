use actix::prelude::*;
use actors::uds_writer::{EventMessage, UdsWriterActor};
use tokio::io::AsyncRead;
use tokio::prelude::Stream;
use tokio_codec::{FramedRead, LinesCodec};
use tornado_collector_common::Collector;
use tornado_collector_json::JsonPayloadCollector;

pub struct RsyslogCollectorActor {
    pub collector: JsonPayloadCollector,
    pub writer_addr: Addr<UdsWriterActor>,
}

#[derive(Message)]
pub struct RsyslogMessage{
    pub json: String
}

impl RsyslogCollectorActor {
    pub fn start_new<S>(source: S, writer_addr: Addr<UdsWriterActor>)
    where
        S: AsyncRead + 'static,
    {
        RsyslogCollectorActor::create(move |ctx| {
            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LinesCodec::new();

            let framed = FramedRead::new(source, codec).map(|msg| RsyslogMessage{ json: msg });
            RsyslogCollectorActor::add_stream(framed, ctx);
            RsyslogCollectorActor { collector: JsonPayloadCollector::new("syslog"), writer_addr }
        });
    }
}

impl Actor for RsyslogCollectorActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("RsyslogCollectorActor started.");
    }
}

/// To use `Framed` with an actor, we have to implement `StreamHandler` trait
impl StreamHandler<RsyslogMessage, std::io::Error> for RsyslogCollectorActor {
    fn handle(&mut self, msg: RsyslogMessage, _ctx: &mut Self::Context) {
        debug!("JsonReaderActor - received msg: [{}]", &msg.json);

        match self.collector.to_event(&msg.json) {
            Ok(event) => self.writer_addr.do_send(EventMessage { event }),
            Err(e) => error!("JsonReaderActor - Cannot unmarshal event from json: {}", e),
        };
    }
}
