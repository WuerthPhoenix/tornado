use actix::prelude::*;
use tokio::prelude::Stream;
use tokio_codec::{FramedRead, LinesCodec};

pub fn start_actix_stdin() {
    System::run(move || {
        StdinActor::create(move |ctx| {
            let codec = LinesCodec::new();
            let source = tokio::io::stdin();
            let framed = FramedRead::new(source, codec).map(LineMessage);
            StdinActor::add_stream(framed, ctx);
            StdinActor {}
        });
    });
}

#[derive(Message)]
pub struct LineMessage(pub String);

pub struct StdinActor {}

impl Actor for StdinActor {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        println!("StdinActor started.");
    }
}

/// To use `Framed` with an actor, we have to implement `StreamHandler` trait
impl StreamHandler<LineMessage, std::io::Error> for StdinActor {
    fn handle(&mut self, msg: LineMessage, _ctx: &mut Self::Context) {
        println!("StdinActor - Received msg: [{}]", &msg.0);
    }
}
