use crate::actors::message::{EventMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use log::*;
use rants::{Address, Client, Connect, Subject};
use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::io::Error;

pub struct NatsPublisherActor {
    subject: Subject,
    client: Client,
}

impl actix::io::WriteHandler<Error> for NatsPublisherActor {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NatsPublisherConfig {
    pub client: NatsClientConfig,
    pub subject: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NatsClientConfig {
    pub addresses: Vec<String>,
}

impl NatsClientConfig {
    pub async fn new_client(&self) -> Result<Client, TornadoError> {
        let addresses = self
            .addresses
            .iter()
            .map(|address| {
                address.to_owned().parse().map_err(|err| TornadoError::ConfigurationError {
                    message: format! {"NatsPublisherActor - Cannot parse address. Err: {}", err},
                })
            })
            .collect::<Result<Vec<Address>, TornadoError>>()?;

        let connect = Connect::new();
        Ok(Client::with_connect(addresses, connect))
    }
}

impl NatsPublisherActor {
    pub async fn start_new(
        config: &NatsPublisherConfig,
        message_mailbox_capacity: usize,
    ) -> Result<Addr<NatsPublisherActor>, TornadoError> {
        let subject = config.subject.parse().map_err(|err| TornadoError::ConfigurationError {
            message: format! {"NatsPublisherActor - Cannot parse subject. Err: {}", err},
        })?;

        let client = config.client.new_client().await?;
        client.connect().await;

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
