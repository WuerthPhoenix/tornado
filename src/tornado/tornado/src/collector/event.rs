use crate::engine::{EventMessage, MatcherActor};
use actix::prelude::*;
use futures::Stream;
use log::*;
use std::io;
use tokio_codec::{FramedRead, LinesCodec};
use tornado_collector_common::Collector;
use tornado_collector_json::JsonEventCollector;
use tornado_common::actors::uds_reader::UdsConnectMessage;

#[derive(Message)]
struct LineFeedMessage {
    pub msg: String,
}

pub struct EventJsonReaderActor {
    pub json_collector: JsonEventCollector,
    pub matcher_addr: Addr<MatcherActor>,
}

impl EventJsonReaderActor {
    pub fn start_new(uds_connect_msg: UdsConnectMessage, matcher_addr: Addr<MatcherActor>) {
        EventJsonReaderActor::create(move |ctx| {
            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LinesCodec::new();

            let framed =
                FramedRead::new(uds_connect_msg.stream, codec).map(|msg| LineFeedMessage { msg });
            EventJsonReaderActor::add_stream(framed, ctx);
            EventJsonReaderActor { json_collector: JsonEventCollector::new(), matcher_addr }
        });
    }
}

impl Actor for EventJsonReaderActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("EventJsonReaderActor started.");
    }
}

/// To use `Framed` with an actor, we have to implement the `StreamHandler` trait
impl StreamHandler<LineFeedMessage, io::Error> for EventJsonReaderActor {
    fn handle(&mut self, msg: LineFeedMessage, _ctx: &mut Self::Context) {
        debug!("JsonReaderActor - received msg: [{}]", &msg.msg);

        match self.json_collector.to_event(&msg.msg) {
            Ok(event) => self.matcher_addr.do_send(EventMessage { event }),
            Err(e) => error!("JsonReaderActor - Cannot unmarshal event from json: {}", e),
        };
    }
}
