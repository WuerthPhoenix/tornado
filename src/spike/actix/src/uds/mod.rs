use actix::prelude::*;
use bytes::BytesMut;
use futures::Future;
use futures::Stream;
use tokio::io::AsyncRead;
use tokio_codec::{Decoder, Encoder, Framed, FramedRead, LinesCodec};
use tokio_uds::UnixStream;
use serde_json;
use std::net;
use std::io;
use tornado_common_api;
use matcher::{EventMessage, MatcherActor};

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

        // For each incoming connection we create `UnixStreamReaderActor` actor
        let matcher_addr = self.matcher_addr.clone();

        UnixStreamReaderActor::create(move |ctx| {

            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LineFeedMessageDecoder {
                lines_codec: LinesCodec::new()
            };

            let framed = Framed::new(msg.0, codec);
            UnixStreamReaderActor::add_stream(framed, ctx);
            UnixStreamReaderActor { matcher_addr }
        });
    }
}

struct UnixStreamReaderActor {
    matcher_addr: Addr<MatcherActor>
}

impl Actor for UnixStreamReaderActor {
    type Context = Context<Self>;
}

#[derive(Message)]
struct LineFeedMessage(pub String);

impl Handler<LineFeedMessage> for UnixStreamReaderActor {
    type Result = ();

    fn handle(&mut self, msg: LineFeedMessage, _: &mut Context<Self>) -> <Self as Handler<LineFeedMessage>>::Result {
        unimplemented!()
    }
}

/// To use `Framed` with an actor, we have to implement `StreamHandler` trait
impl StreamHandler<LineFeedMessage, io::Error> for UnixStreamReaderActor {

    fn handle(&mut self, msg: LineFeedMessage, ctx: &mut Self::Context) {
        debug!("UnixStreamReaderActor - received msg: [{}]", &msg.0);

        match serde_json::from_str::<tornado_common_api::Event>(&msg.0) {
            Ok(event) => self.matcher_addr.do_send(EventMessage{event}),
            Err(e) => error!("UnixStreamReaderActor - Cannot unmarshal event from json: {}", e)
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

