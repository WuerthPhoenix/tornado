use actix::prelude::*;
use log::*;
use serde_json;
use std::io::Error;
use rants::{Client, Subject};
use crate::actors::message::{EventMessage, TornadoCommonActorError};

pub struct NatsPublisherActor {
    restarted: bool,
    subject: Subject,
    client: Client,
}

impl actix::io::WriteHandler<Error> for NatsPublisherActor {}

impl NatsPublisherActor {
    pub async fn start_new<T: 'static + Into<String>>(
        address: T,
        subject: &str,
        tcp_socket_mailbox_capacity: usize,
    ) -> Addr<NatsPublisherActor> {

        let address = address.into().parse().unwrap();
        let client = Client::new(vec![address]);
        // client.connect_mut().await.echo(true);

        let subject = subject.parse().unwrap();

        client.connect().await;

        actix::Supervisor::start(move |ctx: &mut Context<NatsPublisherActor>| {
            ctx.set_mailbox_capacity(tcp_socket_mailbox_capacity);
            NatsPublisherActor { restarted: false, subject, client }
        })
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

