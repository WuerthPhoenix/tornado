use crate::actors::message::{BytesMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use futures::StreamExt;
use log::*;
use tornado_common_api::Event;
use ratsio::{NatsClient, RatsioError, StanClient, StanOptions, NatsClientOptions, StartPosition};
use ratsio::stan_client::StanSubscribe;

pub async fn subscribe_to_nats_streaming<
    F: 'static + FnMut(Event) -> Result<(), TornadoCommonActorError> + Sized + Unpin,
>(
    addresses: &[String],
    subject: &str,
    message_mailbox_capacity: usize,
    callback: F,
) -> Result<(), TornadoError> {

    // Create stan options
    let nats_options = NatsClientOptions::builder()
      //  .tls_required(false)
        .cluster_uris(addresses.iter().cloned().collect::<Vec<String>>())
       // .reconnect_timeout(5u64)
        .build()
        .unwrap();

    let stan_options = StanOptions::builder()
        .nats_options(nats_options)
        .cluster_id("test-cluster")
        .client_id("test-client")
        .build()
        .unwrap();

    //Create STAN client
    let stan_client = StanClient::from_options(stan_options).await.map_err(|err| TornadoError::ConfigurationError {
        message: format! {"NatsSubscriberActor - Cannot create Nats Streaming Client. Err: {}", err},
    })?;

    //Subscribe to STAN
    /*
    let sub = StanSubscribe::builder()
        .subject(subject.clone())
        .start_position(StartPosition::First)
        .durable_name(None)
        .manual_acks(false)
        .ack_wait_in_secs(5)
        .build().unwrap();
*/
    let queue_group = None;
    let durable_name = None;
    let (_sid, subscription) = stan_client.subscribe(subject.to_owned(), queue_group, durable_name).await.map_err(|err| {
        TornadoError::ConfigurationError { message: format! {"NatsSubscriberActor - Cannot subscribe to subject [{}]. Err: {}", subject, err} }
    })?;


        NatsStreamingSubscriberActor::create(|ctx| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            ctx.add_message_stream(
                Box::leak(Box::new(subscription))
                    .map(|message| BytesMessage { msg: message.payload }),
            );
            NatsStreamingSubscriberActor { callback }
        });

/*
    let addresses = addresses
        .iter()
        .map(|address| {
            address.to_owned().parse().map_err(|err| TornadoError::ConfigurationError {
                message: format! {"NatsSubscriberActor - Cannot parse address. Err: {}", err},
            })
        })
        .collect::<Result<Vec<Address>, TornadoError>>()?;

    let client = Client::new(addresses);
    client.connect().await;

    let subject = subject.parse().map_err(|err| TornadoError::ConfigurationError {
        message: format! {"NatsSubscriberActor - Cannot parse subject. Err: {}", err},
    })?;

    let (_, subscription) = client.subscribe(&subject, message_mailbox_capacity).await.map_err(|err| {
        TornadoError::ConfigurationError { message: format! {"NatsSubscriberActor - Cannot subscribe to subject [{}]. Err: {}", subject, err} }
    })?;

    NatsStreamingSubscriberActor::create(|ctx| {
        ctx.set_mailbox_capacity(message_mailbox_capacity);
        ctx.add_message_stream(
            Box::leak(Box::new(subscription))
                .map(|message| BytesMessage { msg: message.into_payload() }),
        );
        NatsStreamingSubscriberActor { callback }
    });
*/
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
