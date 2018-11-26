use actix::prelude::*;
use engine::{EventMessage, MatcherActor};
use futures::Stream;
use reader::uds::UdsConnectMessage;
use std::io;
use std::thread;
use tokio_codec::{LinesCodec, FramedRead};
use tornado_collector_common::Collector;
use tornado_collector_json::JsonCollector;

#[derive(Message)]
pub struct LineFeedMessage(pub String);

pub struct JsonReaderActor {
    pub json_collector: JsonCollector,
    pub matcher_addr: Addr<MatcherActor>,
}

impl JsonReaderActor {
    pub fn start_new(uds_connect_msg: UdsConnectMessage, matcher_addr: Addr<MatcherActor>) {
        JsonReaderActor::create(move |ctx| {
            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LinesCodec::new();

            let framed = FramedRead::new(uds_connect_msg.0, codec)
                .map(LineFeedMessage);
            JsonReaderActor::add_stream(framed, ctx);
            JsonReaderActor { json_collector: JsonCollector::new(), matcher_addr }
        });
    }
}

impl Actor for JsonReaderActor {
    type Context = Context<Self>;
}

/// To use `Framed` with an actor, we have to implement `StreamHandler` trait
impl StreamHandler<LineFeedMessage, io::Error> for JsonReaderActor {
    fn handle(&mut self, msg: LineFeedMessage, _ctx: &mut Self::Context) {
        debug!("JsonReaderActor - {:?} - received msg: [{}]", thread::current().name(), &msg.0);

        match self.json_collector.to_event(&msg.0) {
            Ok(event) => self.matcher_addr.do_send(EventMessage { event }),
            Err(e) => error!(
                "JsonReaderActor - {:?} - Cannot unmarshal event from json: {}",
                thread::current().name(),
                e
            ),
        };
    }
}
