use actix::prelude::*;
use log::*;
use serde_json;
use std::io::Error;
use rants::{Client, Subject, Address};
use crate::actors::message::{EventMessage, TornadoCommonActorError};
use crate::TornadoError;

pub struct NatsPublisherActor {
    restarted: bool,
    subject: Subject,
    client: Client,
}

impl actix::io::WriteHandler<Error> for NatsPublisherActor {}

impl NatsPublisherActor {
    pub async fn start_new(
        addresses: &[String],
        subject: &str,
        tcp_socket_mailbox_capacity: usize,
    ) -> Result<Addr<NatsPublisherActor>, TornadoError> {

        let addresses = addresses.iter().map(|address| {
            address.to_owned().parse().map_err(|err| {
                TornadoError::ConfigurationError { message: format! {"NatsSubscriberActor - Cannot parse address. Err: {}", err} }
            })
        }).collect::<Result<Vec<Address>, TornadoError>>()?;

        let client = Client::new(addresses);

        let subject = subject.parse().map_err(|err| {
            TornadoError::ConfigurationError { message: format! {"NatsPublisherActor - Cannot parse subject. Err: {}", err} }
        })?;

        client.connect().await;

        Ok(actix::Supervisor::start(move |ctx: &mut Context<NatsPublisherActor>| {
            ctx.set_mailbox_capacity(tcp_socket_mailbox_capacity);
            NatsPublisherActor { restarted: false, subject, client }
        }))
    }
}

impl Actor for NatsPublisherActor {
    type Context = Context<Self>;
}

impl actix::Supervised for NatsPublisherActor {
    fn restarting(&mut self, _ctx: &mut Context<NatsPublisherActor>) {
        info!("Restarting NatsPublisherActor");
        self.restarted = true;
    }
}

impl Handler<EventMessage> for NatsPublisherActor {
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, msg: EventMessage, _ctx: &mut Context<Self>) -> Self::Result {
        trace!("NatsPublisherActor - {:?} - received new event", &msg.event);

        let event = serde_json::to_vec(&msg.event).map_err(|err| {
            TornadoCommonActorError::SerdeError { message: format! {"{}", err} }
        })?;

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

