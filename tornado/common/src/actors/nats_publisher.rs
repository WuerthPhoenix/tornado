use crate::actors::message::{EventMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use async_nats::{Connection, Options};
use log::*;
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

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
        path_to_pkcs12_bundle: String,
        pkcs12_bundle_password: String,
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
                path_to_pkcs12_bundle: path_to_pkcs_bundle,
                pkcs12_bundle_password: pkcs_password,
                path_to_root_certificate,
            } => {
                let implement_nats_tls = 0;
                unimplemented!("TLS NOT IMPLEMENTED YET. To be fixed in TOR-314");
                /*
                let mut connect = Connect::new();
                connect.tls_required(true);
                let mut client = Client::with_connect(addresses, connect);

                let mut tls_connector_builder = TlsConnector::builder();

                // Load root certificate, if path is configured
                if let Some(path_to_root_certificate) = path_to_root_certificate {
                    let mut buf = vec![];
                    read_file(&path_to_root_certificate, &mut buf).await?;
                    let root_ca_certificate = Certificate::from_pem(&buf).map_err(|err| {
                        TornadoError::ConfigurationError {
                            message: format!(
                                "Error while constructing certificate from pem file {}. Err: {}",
                                path_to_root_certificate, err
                            ),
                        }
                    })?;
                    tls_connector_builder.add_root_certificate(root_ca_certificate);
                };

                let mut buf = vec![];
                read_file(&path_to_pkcs_bundle, &mut buf).await?;
                let identity =
                    Identity::from_pkcs12(&buf, pkcs_password.as_str()).map_err(|err| {
                        TornadoError::ConfigurationError {
                            message: format!(
                                "Error while constructing identity from pkcs12 file {}. Err: {}",
                                path_to_pkcs_bundle, err
                            ),
                        }
                    })?;

                let tls_connector =
                    tls_connector_builder.identity(identity).build().map_err(|err| {
                        TornadoError::ConfigurationError {
                            message: format!("Error while building tls connector. Err: {}", err),
                        }
                    })?;

                client.set_tls_config(tls_connector).await;
                client

                 */
            }
            NatsClientAuth::None => {
                info!("Open Nats connection (without TLS) to [{}]", addresses);
                Options::new().connect(&addresses).await.map_err(|err| {
                    TornadoError::ConfigurationError {
                        message: format!("Error while building tls connector. Err: {}", err),
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

async fn read_file(path: &str, buf: &mut Vec<u8>) -> Result<usize, TornadoError> {
    let mut file = File::open(path).await.map_err(|err| TornadoError::ConfigurationError {
        message: format!("Error while opening file {}. Err: {}", path, err),
    })?;
    file.read_to_end(buf).await.map_err(|err| TornadoError::ConfigurationError {
        message: format!("Error while reading file {}. Err: {}", path, err),
    })
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
                error!("NatsPublisherActor - Error sending event to NATS. Err: {}", e);
            };
            debug!("NatsPublisherActor - Publish event to NATS succeeded");
        });

        Ok(())
    }
}
