use actix::prelude::*;
use bytes::BytesMut;
use futures::Stream;
use std::fs;
use std::io;
use std::path::Path;
use std::thread;
use tokio_codec::{Decoder, Encoder, LinesCodec};
use tokio_uds::*;

pub fn listen_to_uds_socket<P: AsRef<Path>, F: 'static + FnMut(UdsConnectMessage) -> () + Sized>(
    path: P,
    callback: F,
) {
    let listener = match UnixListener::bind(&path) {
        Ok(m) => m,
        Err(_) => {
            fs::remove_file(&path).unwrap();
            UnixListener::bind(&path).unwrap()
        }
    };

    UdsServerActor::create(|ctx| {
        ctx.add_message_stream(listener.incoming().map_err(|e| panic!("err={:?}", e)).map(
            |stream| {
                //let addr = stream.peer_addr().unwrap();
                UdsConnectMessage(stream)
            },
        ));
        UdsServerActor { callback }
    });
}

struct UdsServerActor<F>
where
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
{
    pub callback: F,
}

impl<F> Actor for UdsServerActor<F>
where
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
{
    type Context = Context<Self>;
}

#[derive(Message)]
pub struct UdsConnectMessage(pub UnixStream);

/// Handle stream of UnixStream's
impl<F> Handler<UdsConnectMessage> for UdsServerActor<F>
where
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
{
    type Result = ();

    fn handle(&mut self, msg: UdsConnectMessage, _: &mut Context<Self>) {
        info!("UdsServerActor - {:?} - new client connected", thread::current().name());
        (&mut self.callback)(msg);
    }
}

#[derive(Message)]
pub struct LineFeedMessage(pub String);

pub struct LineFeedMessageDecoder {
    lines_codec: LinesCodec,
}

impl Default for LineFeedMessageDecoder {
    fn default() -> Self {
        LineFeedMessageDecoder { lines_codec: LinesCodec::new() }
    }
}

impl LineFeedMessageDecoder {
    pub fn new() -> LineFeedMessageDecoder {
        Default::default()
    }
}

impl Decoder for LineFeedMessageDecoder {
    type Item = LineFeedMessage;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<LineFeedMessage>, io::Error> {
        let result = self.lines_codec.decode(src)?;
        Ok(result.map(LineFeedMessage))
    }
}

impl Encoder for LineFeedMessageDecoder {
    type Item = LineFeedMessage;
    type Error = io::Error;

    fn encode(
        &mut self,
        item: <Self as Encoder>::Item,
        dst: &mut BytesMut,
    ) -> Result<(), <Self as Encoder>::Error> {
        self.lines_codec.encode(item.0, dst)
    }
}
