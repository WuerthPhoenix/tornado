use actix::prelude::*;
use bytes::BytesMut;
use tokio_codec::{Decoder, Encoder, Framed, LinesCodec};
use tokio_uds::UnixStream;
use std::io;
use std::thread;
use matcher::{EventMessage, MatcherActor};
use tornado_collector_common::Collector;
use tornado_collector_json::JsonCollector;

pub struct UdsServerActor {
    pub matcher_addr: Addr<MatcherActor>,
}

impl Actor for UdsServerActor {
    type Context = Context<Self>;
}

#[derive(Message)]
pub struct UdsConnectMessage(pub UnixStream);

/// Handle stream of UnixStream's
impl Handler<UdsConnectMessage> for UdsServerActor {
    type Result = ();

    fn handle(&mut self, msg: UdsConnectMessage, _: &mut Context<Self>) {

        info!("UdsServerActor - {:?} - new client connected", thread::current().name());

        // For each incoming connection we create `UnixStreamReaderActor` actor
        let matcher_addr = self.matcher_addr.clone();

        JsonReaderActor::create(move |ctx| {

            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LineFeedMessageDecoder {
                lines_codec: LinesCodec::new()
            };

            let framed = Framed::new(msg.0, codec);
            JsonReaderActor::add_stream(framed, ctx);
            JsonReaderActor {
                json_collector: JsonCollector::new(),
                matcher_addr }
        });
    }
}

struct JsonReaderActor {
    json_collector: JsonCollector,
    matcher_addr: Addr<MatcherActor>
}

impl Actor for JsonReaderActor {
    type Context = Context<Self>;
}

#[derive(Message)]
struct LineFeedMessage(pub String);

/// To use `Framed` with an actor, we have to implement `StreamHandler` trait
impl StreamHandler<LineFeedMessage, io::Error> for JsonReaderActor {

    fn handle(&mut self, msg: LineFeedMessage, _ctx: &mut Self::Context) {
        info!("UnixStreamReaderActor - {:?} - received msg: [{}]", thread::current().name(), &msg.0);

        match self.json_collector.to_event(&msg.0) {
            Ok(event) => self.matcher_addr.do_send(EventMessage{event}),
            Err(e) => error!("JsonReaderActor - {:?} - Cannot unmarshal event from json: {}", thread::current().name(), e)
        };
    }
}


struct LineFeedMessageDecoder {
    lines_codec: LinesCodec
}

impl Decoder for LineFeedMessageDecoder {
    type Item = LineFeedMessage;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<LineFeedMessage>, io::Error> {
        let result = self.lines_codec.decode(src)?;
        Ok(result.map(|line| LineFeedMessage(line) ))
    }
}


impl Encoder for LineFeedMessageDecoder {
    type Item = LineFeedMessage;
    type Error = io::Error;

    fn encode(&mut self, item: <Self as Encoder>::Item, dst: &mut BytesMut) -> Result<(), <Self as Encoder>::Error> {
        self.lines_codec.encode(item.0, dst)
    }
}
