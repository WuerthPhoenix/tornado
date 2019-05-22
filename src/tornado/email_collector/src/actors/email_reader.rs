use actix::prelude::*;
use futures::Stream;
use log::*;
use std::io;
use tokio::prelude::AsyncRead;
use tokio_codec::{FramedRead, LinesCodec};
use tornado_collector_common::Collector;
use tornado_collector_email::EmailEventCollector;
use tornado_common_api::Event;
use tornado_common::actors::message::{AsyncReadMessage, StringMessage};

pub struct EmailReaderActor {
    pub email_collector: EmailEventCollector
}

impl EmailReaderActor {
    pub fn start_new() -> EmailReaderActor {
        EmailReaderActor::create(move |ctx| {
            EmailReaderActor { email_collector: EmailEventCollector::new()}
        });
    }
}

impl Actor for EmailReaderActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("EmailReaderActor started.");
    }
}

impl<F: Fn(Event) + 'static> StreamHandler<StringMessage, io::Error> for EmailReaderActor<F> {
    tokio::io::read_to_end()
    fn handle(&mut self, msg: StringMessage, _ctx: &mut Self::Context) {
        debug!("JsonReaderActor - received msg: [{}]", &msg.msg);

        match self.email_collector.to_event(&msg.msg) {
            Ok(event) => (self.callback)(event),
            Err(e) => error!("JsonReaderActor - Cannot unmarshal event from json: {}", e),
        };
    }
}
