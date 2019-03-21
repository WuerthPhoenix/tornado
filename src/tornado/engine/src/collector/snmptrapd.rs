use crate::engine::{EventMessage, MatcherActor};
use actix::prelude::*;
use futures::Stream;
use log::*;
use std::io;
use tokio_codec::{FramedRead, LinesCodec};
use tornado_collector_common::Collector;
use tornado_collector_snmptrapd::SnmptradpCollector;
use tornado_common::actors::message::AsyncReadMessage;
use tokio::net::UnixStream;

#[derive(Message)]
struct LineFeedMessage {
    pub msg: String,
}

pub struct SnmptrapdJsonReaderActor {
    pub collector: SnmptradpCollector,
    pub matcher_addr: Addr<MatcherActor>,
}

impl SnmptrapdJsonReaderActor {
    pub fn start_new(uds_connect_msg: AsyncReadMessage<UnixStream>, matcher_addr: Addr<MatcherActor>) {
        SnmptrapdJsonReaderActor::create(move |ctx| {
            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LinesCodec::new();

            let framed =
                FramedRead::new(uds_connect_msg.stream, codec).map(|msg| LineFeedMessage { msg });
            SnmptrapdJsonReaderActor::add_stream(framed, ctx);
            SnmptrapdJsonReaderActor { collector: SnmptradpCollector::new(), matcher_addr }
        });
    }
}

impl Actor for SnmptrapdJsonReaderActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("SnmptrapdJsonReaderActor started.");
    }
}

/// To use `Framed` with an actor, we have to implement the `StreamHandler` trait
impl StreamHandler<LineFeedMessage, io::Error> for SnmptrapdJsonReaderActor {
    fn handle(&mut self, msg: LineFeedMessage, _ctx: &mut Self::Context) {
        debug!("SnmptrapdJsonReaderActor - received msg: [{}]", &msg.msg);

        match self.collector.to_event(&msg.msg) {
            Ok(event) => self.matcher_addr.do_send(EventMessage { event }),
            Err(e) => error!("SnmptrapdJsonReaderActor - Cannot unmarshal event from json: {}", e),
        };
    }
}
