use crate::actors::message::{EventMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use async_nats::{Connection, Options};
use log::*;
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::sync::Arc;

pub struct NatsPublisherActor {
    config: Arc<NatsPublisherConfig>,
    client: Arc<Connection>,
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
    pub async fn new_client(&self) -> Result<Connection, TornadoError> {
        let addresses = self.addresses.join(",");

        let auth = self.get_auth();

        let client = match auth {
            NatsClientAuth::Tls {
                certificate_path,
                private_key_path,
                path_to_root_certificate,
            } => {
                info!("NatsClientConfig - Open Nats connection (with TLS) to [{}]", addresses);
                let mut options = Options::new()
                    .client_cert(certificate_path, private_key_path)
                    .tls_required(true);

                if let Some(path_to_root_certificate) = path_to_root_certificate {
                    debug!("NatsClientConfig - Trusting CA: {}", path_to_root_certificate);
                    options = options.add_root_certificate(path_to_root_certificate)
                }

                options.connect(&addresses).await.map_err(|err| {
                    TornadoError::ConfigurationError {
                        message: format!(
                            "Error during connection to NATS with TLS (with TLS). Err: {:?}",
                            err
                        ),
                    }
                })?
            }
            NatsClientAuth::None => {
                info!("NatsClientConfig - Open Nats connection (without TLS) to [{}]", addresses);
                Options::new().connect(&addresses).await.map_err(|err| {
                    TornadoError::ConfigurationError {
                        message: format!(
                            "Error during connection to NATS (without TLS). Err: {:?}",
                            err
                        ),
                    }
                })?
            }
        };

        Ok(client)
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
        let client = config.client.new_client().await?;

        Ok(actix::Supervisor::start(move |ctx: &mut Context<NatsPublisherActor>| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            NatsPublisherActor { config: Arc::new(config), client: Arc::new(client) }
        }))
    }
}

impl Actor for NatsPublisherActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!(
            "NatsPublisherActor started. Connected to NATS address(es): {:?}",
            self.config.client.addresses
        );
    }
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
        let config = self.config.clone();

        actix::spawn(async move {
            debug!("NatsPublisherActor - Publish event to NATS");
            if let Err(e) = client.publish(&config.subject, &event).await {
                error!("NatsPublisherActor - Error sending event to NATS. Err: {:?}", e);
            };
            debug!("NatsPublisherActor - Publish event to NATS succeeded");
        });

        Ok(())
    }
}
