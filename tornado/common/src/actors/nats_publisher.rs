use crate::actors::message::{EventMessage, ResetActorMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use log::*;
use native_tls::{Certificate, Identity, TlsConnector};
use rants::{generate_delay_generator, Address, Client, Connect, Subject};
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::sync::Arc;
use tokio::fs::File;
use tokio::prelude::*;
use tokio::time;
use tokio::time::Duration;

pub struct NatsPublisherActor {
    restarted: bool,
    subject: Arc<Subject>,
    client_config: Arc<NatsClientConfig>,
    client: Option<Client>,
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

        let auth = self.get_auth();

        let client = match auth {
            NatsClientAuth::Tls {
                path_to_pkcs12_bundle: path_to_pkcs_bundle,
                pkcs12_bundle_password: pkcs_password,
                path_to_root_certificate,
            } => {
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

                client.set_tls_connector(tls_connector).await;
                client
            }
            NatsClientAuth::None => {
                let connect = Connect::new();
                Client::with_connect(addresses, connect)
            }
        };
        {
            let mut delay_generator = client.delay_generator_mut().await;
            *delay_generator = generate_delay_generator(
                3,
                Duration::from_secs(0),
                Duration::from_secs(5),
                Duration::from_secs(10),
            );
        }

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
    pub fn start_new(
        config: NatsPublisherConfig,
        message_mailbox_capacity: usize,
    ) -> Result<Addr<NatsPublisherActor>, TornadoError> {
        let subject =
            Arc::new(config.subject.parse().map_err(|err| TornadoError::ConfigurationError {
                message: format! {"NatsPublisherActor - Cannot parse subject. Err: {}", err},
            })?);

        Ok(actix::Supervisor::start(move |ctx: &mut Context<NatsPublisherActor>| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            NatsPublisherActor {
                restarted: false,
                subject,
                client_config: Arc::new(config.client),
                client: None,
            }
        }))
    }
}

impl Actor for NatsPublisherActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(
            "NatsPublisherActor started. Attempting connection to server [{:?}]",
            &self.client_config.addresses
        );

        let mut delay_until = time::Instant::now();
        if self.restarted {
            delay_until += time::Duration::new(1, 0)
        }

        let client_config = self.client_config.clone();
        let current_client = self.client.clone();

        ctx.wait(
            async move {
                if let Some(client) = current_client {
                    client.disconnect().await;
                }

                time::delay_until(delay_until).await;
                match client_config.new_client().await {
                    Ok(client) => {
                        client.connect().await;
                        Ok(client)
                    }
                    Err(e) => Err(e),
                }
            }
            .into_actor(self)
            .map(move |client, act, ctx| match client {
                Ok(client) => {
                    info!(
                        "NatsPublisherActor connected to server [{:?}]",
                        &act.client_config.addresses
                    );
                    act.client = Some(client);
                }
                Err(err) => {
                    act.client = None;
                    warn!("NatsPublisherActor connection failed. Err: {}", err);
                    ctx.stop();
                }
            }),
        );
    }
}

impl actix::Supervised for NatsPublisherActor {
    fn restarting(&mut self, _ctx: &mut Context<NatsPublisherActor>) {
        info!("Restarting NatsPublisherActor");
        self.restarted = true;
    }
}

impl Handler<EventMessage> for NatsPublisherActor {
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, msg: EventMessage, ctx: &mut Context<Self>) -> Self::Result {
        trace!("NatsPublisherActor - {:?} - received new event", &msg.event);

        let event = serde_json::to_vec(&msg.event)
            .map_err(|err| TornadoCommonActorError::SerdeError { message: format! {"{}", err} })?;

        match &mut self.client {
            Some(client) => {
                let client = client.clone();
                let subject = self.subject.clone();
                let address = ctx.address();
                actix::spawn(async move {
                    debug!("NatsPublisherActor - Publish event to NATS");
                    if let Err(e) = client.publish(&subject, &event).await {
                        error!("NatsPublisherActor - Error sending event to NATS. Err: {}", e);
                        if let rants::error::Error::NotConnected = e {
                            warn!(
                                "NatsPublisherActor - Connection not available. Resending message."
                            );
                            address.do_send(ResetActorMessage { payload: Some(msg) });
                        }
                    };
                });
                Ok(())
            }
            None => {
                warn!("NatsPublisherActor - Connection not available. Restart Actor.");
                ctx.address().do_send(ResetActorMessage { payload: Some(msg) });
                Ok(())
            }
        }
    }
}

impl Handler<ResetActorMessage<Option<EventMessage>>> for NatsPublisherActor {
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(
        &mut self,
        msg: ResetActorMessage<Option<EventMessage>>,
        ctx: &mut Context<Self>,
    ) -> Self::Result {
        trace!("NatsPublisherActor - Received reset actor message");
        ctx.stop();
        if let Some(message) = msg.payload {
            ctx.address().do_send(message);
        };
        Ok(())
    }
}
