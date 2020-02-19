use crate::actors::message::{BytesMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use futures::StreamExt;
use log::*;
use rants::{Address, Client};
use tornado_common_api::Event;

pub async fn subscribe_to_nats_streaming<
    F: 'static + FnMut(Event) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
>(
    addresses: &[String],
    subject: &str,
    callback: F,
) -> Result<(), TornadoError> {
    let addresses = addresses
        .iter()
        .map(|address| {
            address.to_owned().parse().map_err(|err| TornadoError::ConfigurationError {
                message: format! {"NatsSubscriberActor - Cannot parse address. Err: {}", err},
            })
        })
        .collect::<Result<Vec<Address>, TornadoError>>()?;

    let subscriber = Client::new(addresses);
    subscriber.connect().await;

    let subject = subject.parse().map_err(|err| TornadoError::ConfigurationError {
        message: format! {"NatsSubscriberActor - Cannot parse subject. Err: {}", err},
    })?;

    let (_, subscription) = subscriber.subscribe(&subject, 1024).await.map_err(|err| {
        TornadoError::ConfigurationError { message: format! {"NatsSubscriberActor - Cannot subscribe to subject [{}]. Err: {}", subject, err} }
    })?;

    NatsStreamingSubscriberActor::create(|ctx| {
        ctx.add_message_stream(
            Box::leak(Box::new(subscription))
                .map(|message| BytesMessage { msg: message.into_payload() }),
        );
        NatsStreamingSubscriberActor { callback }
    });

    Ok(())
}

struct NatsStreamingSubscriberActor<F>
where
    F: 'static + FnMut(Event) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    callback: F,
}

impl<F> Actor for NatsStreamingSubscriberActor<F>
where
    F: 'static + FnMut(Event) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    type Context = Context<Self>;
}

impl<F> Handler<BytesMessage> for NatsStreamingSubscriberActor<F>
where
    F: 'static + FnMut(Event) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, msg: BytesMessage, _: &mut Context<Self>) -> Self::Result {
        trace!("NatsStreamingSubscriberActor - message received");
        let event = serde_json::from_slice(&msg.msg)
            .map_err(|err| TornadoCommonActorError::SerdeError { message: format! {"{}", err} })?;
        trace!("NatsStreamingSubscriberActor - event from message received: {:#?}", event);
        (&mut self.callback)(event)
    }
}
