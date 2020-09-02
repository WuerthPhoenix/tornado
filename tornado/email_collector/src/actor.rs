use actix::dev::ToEnvelope;
use actix::prelude::*;
use log::*;
use std::sync::Arc;
use tokio::prelude::*;
use tornado_collector_common::Collector;
use tornado_collector_email::EmailEventCollector;
use tornado_common::actors::message::{AsyncReadMessage, EventMessage};

pub struct EmailReaderActor<A: Actor + actix::Handler<EventMessage>>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    pub client_addr: Addr<A>,
    pub email_collector: Arc<EmailEventCollector>,
}

impl<A: Actor + actix::Handler<EventMessage>> EmailReaderActor<A>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    pub fn start_new(client_addr: Addr<A>, mailbox_capacity: usize) -> Addr<Self> {
        EmailReaderActor::create(move |ctx| {
            ctx.set_mailbox_capacity(mailbox_capacity);
            EmailReaderActor {
                email_collector: Arc::new(EmailEventCollector::new()),
                client_addr,
            }
        })
    }
}

impl<A: Actor + actix::Handler<EventMessage>> Actor for EmailReaderActor<A>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("EmailReaderActor started.");
    }
}

impl<A: Actor + actix::Handler<EventMessage>, R: AsyncRead + 'static + Unpin>
    Handler<AsyncReadMessage<R>> for EmailReaderActor<A>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    type Result = ();

    fn handle(&mut self, mut msg: AsyncReadMessage<R>, _ctx: &mut Context<Self>) -> Self::Result {
        let tcp = self.client_addr.clone();
        let collector = self.email_collector.clone();
        let fut = async move {
            let mut buf = Vec::new();
            msg.stream.read_to_end(&mut buf).await.unwrap();

            if log_enabled!(Level::Debug) {
                let buf_to_string = String::from_utf8_lossy(&buf);
                debug!("EmailReaderActor - received email:\n{}", buf_to_string);
            }
            match collector.to_event(&buf) {
                Ok(event) => {
                    tcp.try_send(EventMessage { event }).unwrap_or_else(|err| error!("EmailReaderActor -  Error while sending ProcessedEventMessage to TornadoConnectionChannel actor. Error: {}", err));
                }
                Err(e) => error!("Error processing incoming email. Err: {}", e),
            };
        };

        actix::spawn(fut);
    }
}
