use crate::actors::message::{EventMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use log::*;
use ratsio::{NatsClient, RatsioError, StanClient, StanOptions, NatsClientOptions, StartPosition};
use serde_json;
use std::io::Error;
use std::sync::Arc;

pub struct NatsPublisherActor {
    subject: String,
    client: Arc<StanClient>,
}

impl actix::io::WriteHandler<Error> for NatsPublisherActor {}

impl NatsPublisherActor {
    pub async fn start_new(
        addresses: &[String],
        subject: &str,
        message_mailbox_capacity: usize,
    ) -> Result<Addr<NatsPublisherActor>, TornadoError> {

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
            .client_id("test-client_pub")
            .build()
            .unwrap();

        //Create STAN client
        let client = StanClient::from_options(stan_options).await.map_err(|err| TornadoError::ConfigurationError {
            message: format! {"NatsSubscriberActor - Cannot create Nats Streaming Client. Err: {}", err},
        })?;

        let subject = subject.to_owned();

        Ok(actix::Supervisor::start(move |ctx: &mut Context<NatsPublisherActor>| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            NatsPublisherActor { subject, client }
        }))
    }
}

impl Actor for NatsPublisherActor {
    type Context = Context<Self>;
}

impl actix::Supervised for NatsPublisherActor {
    fn restarting(&mut self, _ctx: &mut Context<NatsPublisherActor>) {
        info!("Restarting NatsPublisherActor");
    }
}

impl Handler<EventMessage> for NatsPublisherActor {
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, msg: EventMessage, _ctx: &mut Context<Self>) -> Self::Result {
        trace!("NatsPublisherActor - {:?} - received new event", &msg.event);

        let event = serde_json::to_vec(&msg.event)
            .map_err(|err| TornadoCommonActorError::SerdeError { message: format! {"{}", err} })?;

        let client = self.client.clone();
        let subject = self.subject.clone();
        actix::spawn(async move {
            debug!("NatsPublisherActor - Publish event to NATS");
            if let Err(e) = client.publish(&subject, &event).await {
                error!("NatsPublisherActor - Error sending event to NATS. Err: {}", e)
            };
        });

        Ok(())
    }
}
