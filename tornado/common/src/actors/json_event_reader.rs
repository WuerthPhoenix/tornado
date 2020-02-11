use crate::actors::message::AsyncReadMessage;

use actix::prelude::*;
use log::*;
use tokio::prelude::AsyncRead;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};
use tornado_collector_common::Collector;
use tornado_collector_json::JsonEventCollector;
use tornado_common_api::Event;

/*
use std::iter::Iterator;
use actix::fut::ActorFuture;
use actix::fut::ActorStream;
*/

pub struct JsonEventReaderActor<F: Fn(Event) + 'static + Unpin> {
    pub json_collector: JsonEventCollector,
    pub callback: F,
}

impl<F: Fn(Event) + 'static + Unpin> JsonEventReaderActor<F> {
    pub fn start_new<R: AsyncRead + 'static>(connect_msg: AsyncReadMessage<R>, callback: F) {
        JsonEventReaderActor::create(move |ctx| {
            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LinesCodec::new();

            let framed = FramedRead::new(connect_msg.stream, codec);
            ctx.add_stream(framed);
            //JsonEventReaderActor::add_stream(framed, ctx);
            JsonEventReaderActor { json_collector: JsonEventCollector::new(), callback }
        });
    }
}

impl<F: Fn(Event) + 'static + Unpin> Actor for JsonEventReaderActor<F> {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("EventJsonReaderActor started.");
    }
}

/// To use `Framed` with an actor, we have to implement the `StreamHandler` trait
impl<F: Fn(Event) + 'static + Unpin> StreamHandler<Result<String, LinesCodecError>>
    for JsonEventReaderActor<F>
{
    fn handle(&mut self, msg: Result<String, LinesCodecError>, _ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                debug!("JsonReaderActor - received json message: [{}]", msg);
                match self.json_collector.to_event(&msg) {
                    Ok(event) => (self.callback)(event),
                    Err(e) => error!("JsonReaderActor - Cannot unmarshal event from json: {}", e),
                };
            }
            Err(err) => {
                error!("JsonEventReaderActor stream error. Err: {}", err);
            }
        }
    }
}
