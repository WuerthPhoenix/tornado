use crate::actors::message::{EventMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use async_nats::{Connection, Options};
use log::*;
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::ops::Deref;
use std::sync::Arc;
use tokio::time;

pub struct NatsPublisherActor {
    config: Arc<NatsPublisherConfig>,
    nats_connection: Arc<Option<Connection>>,
}

impl actix::io::WriteHandler<Error> for NatsPublisherActor {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NatsPublisherConfig {
    pub client: NatsClientConfig,
    pub subject: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum NatsClientAuth {
    None,
    Tls {
        certificate_path: String,
        private_key_path: String,
        path_to_root_certificate: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NatsClientConfig {
    pub addresses: Vec<String>,
    pub auth: Option<NatsClientAuth>,
}

impl NatsClientConfig {
    pub async fn new_client(&self) -> Connection {
        let addresses = self.addresses.join(",");

        let auth = self.get_auth();

        loop {
            let mut options = Options::new()
                .disconnect_callback(|| {
                    error!("NatsClientConfig - connection to NATS server was lost")
                })
                .reconnect_callback(|| {
                    info!("NatsClientConfig - connection to NATS server was restored")
                })
                .max_reconnects(None);
            match auth {
                NatsClientAuth::Tls {
                    certificate_path,
                    private_key_path,
                    path_to_root_certificate,
                } => {
                    info!("NatsClientConfig - Open Nats connection (with TLS) to [{}]", addresses);
                    options =
                        options.client_cert(certificate_path, private_key_path).tls_required(true);

                    if let Some(path_to_root_certificate) = path_to_root_certificate {
                        debug!("NatsClientConfig - Trusting CA: {}", path_to_root_certificate);
                        options = options.add_root_certificate(path_to_root_certificate)
                    }
                }
                NatsClientAuth::None => {
                    info!(
                        "NatsClientConfig - Open Nats connection (without TLS) to [{}]",
                        addresses
                    );
                }
            };
            match options.connect(&addresses).await {
                Err(connection_error) => {
                    error!("Error during connection to NATS. Err: {}", connection_error);
                    time::delay_until(time::Instant::now() + time::Duration::from_secs(5)).await;
                }
                Ok(connection) => {
                    return connection;
                }
            };
        }
    }

    fn get_auth(&self) -> &NatsClientAuth {
        match &self.auth {
            None => &NatsClientAuth::None,
            Some(auth) => &auth,
        }
    }
}

impl NatsPublisherActor {
    pub async fn start_new(
        config: NatsPublisherConfig,
        message_mailbox_capacity: usize,
    ) -> Result<Addr<NatsPublisherActor>, TornadoError> {
        Ok(actix::Supervisor::start(move |ctx: &mut Context<NatsPublisherActor>| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            NatsPublisherActor { config: Arc::new(config), nats_connection: Arc::new(None) }
        }))
    }
}

impl Actor for NatsPublisherActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(
            "NatsPublisherActor started. Connected to NATS address(es): {:?}",
            self.config.client.addresses
        );

        let client_config = self.config.client.clone();
        ctx.wait(async move { client_config.new_client().await }.into_actor(self).map(
            move |connection, actor, _ctx| actor.nats_connection = Arc::new(Some(connection)),
        ));
    }
}

impl actix::Supervised for NatsPublisherActor {
    fn restarting(&mut self, _ctx: &mut Context<NatsPublisherActor>) {
        info!("Restarting NatsPublisherActor");
    }
}

impl Handler<EventMessage> for NatsPublisherActor {
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, msg: EventMessage, ctx: &mut Context<Self>) -> Self::Result {
        trace!("NatsPublisherActor - {:?} - received new event", &msg.event);
        if let Some(connection) = self.nats_connection.deref() {
            let event = serde_json::to_vec(&msg.event).map_err(|err| {
                TornadoCommonActorError::SerdeError { message: format! {"{}", err} }
            })?;

            let client = connection.clone();
            let config = self.config.clone();

            actix::spawn(async move {
                debug!("NatsPublisherActor - Publish event to NATS");
                let res = client.publish(&config.subject, &event).await;
                match res {
                    Ok(_) => error!("NatsPublisherActor - Publish event to NATS succeeded"),
                    Err(e) => {
                        error!("NatsPublisherActor - Error sending event to NATS. Err: {}", e)
                    }
                }
            });
        } else {
            // This should be rare because while establishing connection to NATS, events are not
            // processed by the actor
            warn!("NatsPublisherActor - Processing event but NATS connection not yet established. Reprocessing event ...");
            ctx.notify_later(msg, std::time::Duration::from_secs(1));
        }

        Ok(())
    }
}
