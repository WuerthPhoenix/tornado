use crate::actors::message::{EventMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use log::*;
use ratsio::{NatsClientOptions, StanClient, StanOptions};
use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::io::Error;
use std::sync::Arc;

pub struct NatsPublisherActor {
    subject: String,
    client: Arc<StanClient>,
}

impl actix::io::WriteHandler<Error> for NatsPublisherActor {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StanPublisherConfig {
    pub base: StanBaseConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StanBaseConfig {
    pub addresses: Vec<String>,
    pub subject: String,
    pub cluster_id: String,
    pub client_id: String,
}

impl StanBaseConfig {
    pub async fn new_client(&self) -> Result<Arc<StanClient>, TornadoError> {
        // Create stan options
        let mut nats_options = NatsClientOptions::default();
        nats_options.cluster_uris = self.addresses.clone().into();
        let stan_options =
            StanOptions::with_options(nats_options, &self.cluster_id, &self.client_id);

        //Create STAN client
        StanClient::from_options(stan_options).await.map_err(|err| {
            TornadoError::ConfigurationError {
                message: format! {"StanConfig - Cannot create Nats Streaming Client. Err: {}", err},
            }
        })
    }
}

impl NatsPublisherActor {
    pub async fn start_new(
        config: &StanPublisherConfig,
        message_mailbox_capacity: usize,
    ) -> Result<Addr<NatsPublisherActor>, TornadoError> {
        let client = config.base.new_client().await?;
        let subject = config.base.subject.to_owned();
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
