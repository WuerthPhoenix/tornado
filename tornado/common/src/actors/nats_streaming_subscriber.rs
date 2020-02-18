use crate::actors::message::{EventMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use futures::StreamExt;
use log::*;
use rants::Client;

pub async fn subscribe_to_nats_streaming<
    P: 'static + Into<String>,
    F: 'static + FnMut(EventMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
>(
    address: P,
    subject: &str,
    callback: F,
) -> Result<(), TornadoError> {
    let address = address.into().parse().unwrap();
    let subscriber = Client::new(vec![address]);
    subscriber.connect().await;

    let subject = subject.parse().unwrap();
    let (_, subscription) = subscriber.subscribe(&subject, 1024).await.unwrap();

    NatsStreamingSubscriberActor::create(|ctx| {
        ctx.add_message_stream(Box::leak(Box::new(subscription)).map(|message| {
            EventMessage {
                event: serde_json::from_slice(&message.into_payload()).unwrap()
            }
        }));
        NatsStreamingSubscriberActor { callback }
    });

    Ok(())
}

struct NatsStreamingSubscriberActor<F>
    where
        F: 'static + FnMut(EventMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    callback: F,
}

impl<F> Actor for NatsStreamingSubscriberActor<F>
    where
        F: 'static + FnMut(EventMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    type Context = Context<Self>;
}


impl<F> Handler<EventMessage> for NatsStreamingSubscriberActor<F>
    where
        F: 'static + FnMut(EventMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, msg: EventMessage, _: &mut Context<Self>) -> Self::Result {
        trace!("NatsStreamingSubscriberActor - message received: {:#?}", msg.event);
        (&mut self.callback)(msg)
    }
}
