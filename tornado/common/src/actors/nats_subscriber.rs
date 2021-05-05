use crate::actors::message::{BytesMessage, TornadoCommonActorError};
use crate::actors::nats_publisher::NatsClientConfig;
use crate::TornadoError;
use actix::prelude::*;
use futures::{stream, StreamExt};
use log::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct NatsSubscriberConfig {
    pub client: NatsClientConfig,
    pub subject: String,
}

pub async fn subscribe_to_nats<
    F: 'static + FnMut(BytesMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
>(
    config: NatsSubscriberConfig,
    message_mailbox_capacity: usize,
    callback: F,
) -> Result<(), TornadoError> {

    let client = config.client.new_client().await?;

    let subscription = client.subscribe(&config.subject).await.map_err(|err| {
        TornadoError::ConfigurationError { message: format! {"NatsSubscriberActor - Cannot subscribe to subject [{}]. Err: {}", config.subject, err} }
    })?;

    let message_stream = stream::unfold(subscription, |sub| async {
        sub.next().await.map(|msg| (BytesMessage { msg: msg.data }, sub))
    });

    NatsSubscriberActor::create(|ctx| {
        ctx.set_mailbox_capacity(message_mailbox_capacity);
        ctx.add_message_stream(message_stream);
        NatsSubscriberActor { callback }
    });

    Ok(())
}

struct NatsSubscriberActor<F>
where
    F: 'static + FnMut(BytesMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    callback: F,
}

impl<F> Actor for NatsSubscriberActor<F>
where
    F: 'static + FnMut(BytesMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    type Context = Context<Self>;
}

impl<F> Handler<BytesMessage> for NatsSubscriberActor<F>
where
    F: 'static + FnMut(BytesMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, msg: BytesMessage, _: &mut Context<Self>) -> Self::Result {
        trace!("NatsSubscriberActor - message received");
        (&mut self.callback)(msg)
    }
}
