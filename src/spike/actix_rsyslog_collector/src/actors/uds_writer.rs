use actix::prelude::*;
use serde_json;
use std::io::Error;
use tokio::io::WriteHalf;
use tokio::prelude::*;
use tokio_codec::LinesCodec;
use tokio_uds::*;
use tornado_common_api;

pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

impl Message for EventMessage {
    type Result = Result<(), serde_json::Error>;
}

pub struct UdsWriterActor {
    tx: actix::io::FramedWrite<WriteHalf<UnixStream>, LinesCodec>,
}

impl actix::io::WriteHandler<Error> for UdsWriterActor {}

impl UdsWriterActor {
    pub fn start_new(stream: UnixStream) -> Addr<UdsWriterActor> {
        UdsWriterActor::create(|ctx| {
            let (_r, w) = stream.split();
            UdsWriterActor { tx: actix::io::FramedWrite::new(w, LinesCodec::new(), ctx) }
        })
    }
}

impl Actor for UdsWriterActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        println!("UdsWriterActor started!");
    }
}

impl Handler<EventMessage> for UdsWriterActor {
    type Result = Result<(), serde_json::Error>;

    fn handle(&mut self, msg: EventMessage, _: &mut Context<Self>) -> Self::Result {
        trace!("UdsWriterActor - {:?} - received new event", &msg.event);
        let mut event = serde_json::to_string(&msg.event).unwrap();
        event.push('\n');
        self.tx.write(event);
        Ok(())
    }
}
