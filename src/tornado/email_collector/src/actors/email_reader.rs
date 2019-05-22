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
use std::sync::Arc;
use tornado_common::actors::tcp_client::TcpClientActor;

pub struct EmailReaderActor {
    pub tpc_client_addr: Addr<TcpClientActor>,
    pub email_collector: Arc<EmailEventCollector>,
}

impl EmailReaderActor {
    pub fn start_new(tpc_client_addr: Addr<TcpClientActor>) -> Addr<EmailReaderActor> {
        EmailReaderActor::create(move |_ctx| {
            EmailReaderActor { email_collector: Arc::new(EmailEventCollector::new()), tpc_client_addr}
        })
    }
}

impl Actor for EmailReaderActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("EmailReaderActor started.");
    }
}

impl<R: AsyncRead + 'static> Handler<AsyncReadMessage<R>> for EmailReaderActor {
    type Result = ();

    fn handle(&mut self, msg: AsyncReadMessage<R>, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("EmailReaderActor - received new email");

        let collector = self.email_collector.clone();
        let buf = Vec::new();
        let reader = tokio::io::read_to_end(msg.stream, buf).map(move |(_, buf)| {
            info!("incoming: {:?}", std::str::from_utf8(&buf).unwrap());
            let event = collector.to_event(&buf);
            info!("produced event: {:?}", event.unwrap());
        }).then(|_| Ok(()));

        actix::spawn(reader);

        ()
    }
}
