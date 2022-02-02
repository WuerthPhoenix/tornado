use crate::actors::message::{EventMessage, TornadoCommonActorError, TornadoNatsMessage};
use crate::TornadoError;
use actix::prelude::*;
use async_nats::{Connection, Options};
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Error;
use std::ops::Deref;
use std::rc::Rc;
use tokio::time;
use tornado_common_api::add_metadata_to_span;
use tornado_common_metrics::opentelemetry::global;
use tracing_futures::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

const WAIT_BETWEEN_RESTARTS_SEC: u64 = 10;

pub struct NatsPublisherActor {
    config: NatsPublisherConfig,
    nats_connection: Rc<Option<Connection>>,
    restarted: bool,
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
    pub async fn new_client(&self) -> std::io::Result<Connection> {
        let addresses = self.addresses.join(",");

        let auth = self.get_auth();

        let mut options = Options::new()
            .disconnect_callback(|| error!("NatsClientConfig - connection to NATS server was lost"))
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
                info!("NatsClientConfig - Open Nats connection (without TLS) to [{}]", addresses);
            }
        };
        options.connect(&addresses).await
    }

    fn get_auth(&self) -> &NatsClientAuth {
        match &self.auth {
            None => &NatsClientAuth::None,
            Some(auth) => auth,
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
            NatsPublisherActor { config, nats_connection: Rc::new(None), restarted: false }
        }))
    }
}

impl Actor for NatsPublisherActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(
            "NatsPublisherActor started. Connecting to NATS address(es): {:?}",
            self.config.client.addresses
        );

        let client_config = self.config.client.clone();
        let nats_connection = self.nats_connection.clone();
        let restarted = self.restarted;
        ctx.wait(
            async move {
                if restarted {
                    info!(
                        "NatsPublisherActor was restarted after a failure. Waiting {} seconds before proceeding ...",
                        WAIT_BETWEEN_RESTARTS_SEC
                    );
                    time::sleep(time::Duration::from_secs(WAIT_BETWEEN_RESTARTS_SEC)).await;
                }
                if let Some(connection) = nats_connection.deref() {
                    connection.close().await.unwrap();
                    match connection.close().await {
                        Ok(()) => {debug!(
                            "NatsPublisherActor - Successfully closed previously opened NATS connection."
                        );}
                        Err(err) => {
                            error!("NatsPublisherActor - Error while closing previously opened NATS connection. Err: {:?}", err)
                        }
                    };
                }
                client_config.new_client().await
            }
            .into_actor(self)
                .map(move |client, act, ctx| match client {
                    Ok(client) => {
                        info!(
                            "NatsPublisherActor connected to server [{:?}]",
                            &act.config.client.addresses
                        );
                        act.nats_connection = Rc::new(Some(client));
                    }
                    Err(err) => {
                        act.nats_connection = Rc::new(None);
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

    fn handle(&mut self, mut msg: EventMessage, ctx: &mut Context<Self>) -> Self::Result {
        let mut span = tracing::error_span!("NatsPublisherActor");
        add_metadata_to_span(&mut span, &mut msg.event);

        let _span = span.entered();

        trace!("NatsPublisherActor - Handling Event to be sent to Nats - {:?}", &msg.event);

        let address = ctx.address();

        if let Some(connection) = self.nats_connection.deref() {
            let event = serde_json::to_vec(&msg.event).map_err(|err| {
                TornadoCommonActorError::SerdeError { message: format! {"{}", err} }
            })?;

            let client = connection.clone();
            let config = self.config.clone();

            actix::spawn(async move {
                debug!("NatsPublisherActor - Publishing event to NATS");
                match client.publish(&config.subject, &event).await {
                    Ok(_) => trace!(
                        "NatsPublisherActor - Publish event to NATS succeeded. Event: {:?}",
                        &msg
                    ),
                    Err(e) => {
                        error!("NatsPublisherActor - Error sending event to NATS. Err: {:?}", e);
                        time::sleep(time::Duration::from_secs(1)).await;
                        address.try_send(msg).unwrap_or_else(|err| error!("NatsPublisherActor -  Error while sending event to itself. Error: {}", err));
                    }
                }
            }.instrument(span.exit()));
        } else {
            warn!("NatsPublisherActor - Processing event but NATS connection not yet established. Stopping actor and reprocessing the event ...");
            ctx.stop();
            address.try_send(msg).unwrap_or_else(|err| {
                error!("NatsPublisherActor -  Error while sending event to itself. Err: {:?}", err)
            });
        }

        Ok(())
    }
}

pub async fn wait_for_nats_connection(client_config: &NatsClientConfig) -> Connection {
    loop {
        match client_config.new_client().await {
            Err(connection_error) => {
                error!("Error during connection to NATS. Err: {:?}", connection_error);
                time::sleep(time::Duration::from_secs(5)).await;
            }
            Ok(connection) => {
                info!("NatsClientConfig - Successfully connected to NATS");
                return connection;
            }
        }
    }
}
