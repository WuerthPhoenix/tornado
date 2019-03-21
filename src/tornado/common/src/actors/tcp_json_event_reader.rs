use crate::actors::tcp_server::TcpConnectMessage;

use actix::prelude::*;
use futures::Stream;
use log::*;
use std::io;
use tokio_codec::{FramedRead, LinesCodec};
use tornado_collector_common::Collector;
use tornado_collector_json::JsonEventCollector;
use tornado_common_api::Event;

#[derive(Message)]
struct LineFeedMessage {
    pub msg: String,
}

pub struct JsonEventReaderActor<F: Fn(Event) + 'static> {
    pub json_collector: JsonEventCollector,
    pub callback: F,
}

impl<F: Fn(Event) + 'static> JsonEventReaderActor<F> {
    pub fn start_new(connect_msg: TcpConnectMessage, callback: F) {
        JsonEventReaderActor::create(move |ctx| {
            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LinesCodec::new();

            let framed =
                FramedRead::new(connect_msg.stream, codec).map(|msg| LineFeedMessage { msg });
            JsonEventReaderActor::add_stream(framed, ctx);
            JsonEventReaderActor { json_collector: JsonEventCollector::new(), callback }
        });
    }
}

impl<F: Fn(Event) + 'static> Actor for JsonEventReaderActor<F> {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("EventJsonReaderActor started.");
    }
}

/// To use `Framed` with an actor, we have to implement the `StreamHandler` trait
impl<F: Fn(Event) + 'static> StreamHandler<LineFeedMessage, io::Error> for JsonEventReaderActor<F> {
    fn handle(&mut self, msg: LineFeedMessage, _ctx: &mut Self::Context) {
        debug!("JsonReaderActor - received msg: [{}]", &msg.msg);

        match self.json_collector.to_event(&msg.msg) {
            Ok(event) => (self.callback)(event),
            Err(e) => error!("JsonReaderActor - Cannot unmarshal event from json: {}", e),
        };
    }
}
