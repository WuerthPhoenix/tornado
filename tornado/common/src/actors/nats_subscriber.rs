use crate::actors::message::TornadoCommonActorError;
use crate::actors::nats_publisher::{wait_for_nats_connection, NatsClientConfig};
use crate::TornadoError;
use actix::prelude::*;
use async_nats::{Connection, Message};
use futures_util::stream;
use log::*;
use serde::{Deserialize, Serialize};

#[derive(Message, Debug)]
#[rtype(result = "Result<(), TornadoCommonActorError>")]
pub struct NatsMessage {
    pub msg: Message,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct NatsSubscriberConfig {
    pub client: NatsClientConfig,
    pub subject: String,
}

pub async fn subscribe_to_nats<
    F: 'static + FnMut(NatsMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
>(
    config: NatsSubscriberConfig,
    message_mailbox_capacity: usize,
    callback: F,
) -> Result<(), TornadoError> {
    let client = wait_for_nats_connection(&config.client).await;

    let subscription = client.subscribe(&config.subject).await.map_err(|err| {
        TornadoError::ConfigurationError { message: format! {"NatsSubscriberActor - Cannot subscribe to subject [{}]. Err: {:?}", config.subject, err} }
    })?;

    info!("NatsSubscriberActor - Created Nats subscription to subject [{}]", config.subject);

    let message_stream = stream::unfold(subscription, |sub| async {
        sub.next().await.map(|msg| (NatsMessage { msg }, sub))
    });

    NatsSubscriberActor::create(|ctx| {
        ctx.set_mailbox_capacity(message_mailbox_capacity);
        ctx.add_message_stream(message_stream);
        NatsSubscriberActor { callback, client }
    });

    // Alternative implementation. Do not remove, could be needed for a couple of refactoring.
    /*
    let address = NatsSubscriberActor::create(|ctx| {
        ctx.set_mailbox_capacity(message_mailbox_capacity);
        NatsSubscriberActor {
            callback,
            client
        }
    });

    actix::spawn(async move {
        for message in subscription.next().await {
            trace!("NatsSubscriberActor - Nats subscription received a message");
            if let Err(err) = address.try_send(BytesMessage { msg: message.data }) {
                error!("NatsSubscriberActor - Cannot forward Nats message from subscription to the actor handler. Err: {:?}", err);
            }
        };
    });
    */

    Ok(())
}

struct NatsSubscriberActor<F>
where
    F: 'static + FnMut(NatsMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    callback: F,
    // The client must live as long as the actor, otherwise the connection is dropped when the client is deallocated
    #[allow(dead_code)]
    client: Connection,
}

impl<F> Actor for NatsSubscriberActor<F>
where
    F: 'static + FnMut(NatsMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    type Context = Context<Self>;
}

impl<F> Handler<NatsMessage> for NatsSubscriberActor<F>
where
    F: 'static + FnMut(NatsMessage) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
{
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, msg: NatsMessage, _: &mut Context<Self>) -> Self::Result {
        trace!("NatsSubscriberActor - message received");
        (&mut self.callback)(msg)
    }
}
