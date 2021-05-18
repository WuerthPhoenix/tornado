use crate::actors::message::{EventMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use async_nats::{Connection, Options};
use log::*;
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::sync::Arc;
use tokio::time::Duration;
use tokio::time;
use serde::__private::Option::Some;
use std::ops::Deref;

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
    // pub async fn new_client(&self) -> Connection {
    //     let addresses = self.addresses.join(",");
    //
    //     let auth = self.get_auth();
    //
    //     let mut connection = None;
    //     while connection.is_none() {
    //         let mut options = Options::new()
    //             .disconnect_callback(|| error!("NatsClientConfig - connection to NATS server was lost"))
    //             .reconnect_callback(|| info!("NatsClientConfig - connection to NATS server was restored"))
    //             // Reconnect delay is a backoff capped at 4 secs max
    //             // .reconnect_delay_callback(|c| Duration::from_millis(std::cmp::min((c * 100) as u64, 8000)))
    //             .max_reconnects(None)
    //             .reconnect_buffer_size(1024 * 1024 * 64);
    //         match auth {
    //             NatsClientAuth::Tls {
    //                 certificate_path,
    //                 private_key_path,
    //                 path_to_root_certificate,
    //             } => {
    //                 info!("NatsClientConfig - Open Nats connection (with TLS) to [{}]", addresses);
    //                 options = options
    //                     .client_cert(certificate_path, private_key_path)
    //                     .tls_required(true);
    //
    //                 if let Some(path_to_root_certificate) = path_to_root_certificate {
    //                     debug!("NatsClientConfig - Trusting CA: {}", path_to_root_certificate);
    //                     options = options.add_root_certificate(path_to_root_certificate)
    //                 }
    //             }
    //             NatsClientAuth::None => {
    //                 info!("NatsClientConfig - Open Nats connection (without TLS) to [{}]", addresses);
    //             }
    //         };
    //         connection = match options.connect(&addresses).await {
    //             Ok(connection) => Some(connection),
    //             Err(connection_error) => {
    //                 error!("Error during connection to NATS. Err: {}", connection_error);
    //                 time::delay_until(time::Instant::now() + time::Duration::new(1, 0)).await;
    //                 None
    //             }
    //         }
    //     };
    //
    //     connection.unwrap()
    // }

    fn get_auth(&self) -> &NatsClientAuth {
        match &self.auth {
            None => &NatsClientAuth::None,
            Some(auth) => &auth,
        }
    }
}

// async fn connect_with_retry(client_options: Options, addresses: &str, delay_until: Option<time::Instant>) -> Result<Connection, TornadoError> {
//     let mut delay_until = delay_until.unwrap_or(time::Instant::now());
//     time::delay_until(delay_until).await;
//     let client_options_clone = client_options.clone();
//     let connection_result = client_options_clone.connect(addresses).await;
//     match connection_result {
//         Ok(connection) => Ok(connection),
//         Err(connection_error) => {
//             error!("Error during connection to NATS (without TLS). Err: {}", connection_error);
//             delay_until += time::Duration::new(1, 0);
//             connect_with_retry(client_options, addresses, Some(delay_until)).await
//         }
//     }
// }
//
impl NatsPublisherActor {
    pub async fn start_new(
        config: NatsPublisherConfig,
        message_mailbox_capacity: usize,
    ) -> Result<Addr<NatsPublisherActor>, TornadoError> {
        // let client = config.client.new_client().await;

        Ok(actix::Supervisor::start(move |ctx: &mut Context<NatsPublisherActor>| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            NatsPublisherActor { config: Arc::new(config), nats_connection: Arc::new(None) }
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
        async {
            let auth = self.config.client.get_auth();
            let addresses = self.config.client.addresses.join(",");
            while self.nats_connection.is_none() {
                let mut options = Options::new()
                    .disconnect_callback(|| error!("NatsClientConfig - connection to NATS server was lost"))
                    .reconnect_callback(|| info!("NatsClientConfig - connection to NATS server was restored"))
                    // Reconnect delay is a backoff capped at 4 secs max
                    // .reconnect_delay_callback(|c| Duration::from_millis(std::cmp::min((c * 100) as u64, 8000)))
                    .max_reconnects(None)
                    .reconnect_buffer_size(1024 * 1024 * 64);
                match auth {
                    NatsClientAuth::Tls {
                        certificate_path,
                        private_key_path,
                        path_to_root_certificate,
                    } => {
                        info!("NatsClientConfig - Open Nats connection (with TLS) to [{}]", addresses);
                        options = options
                            .client_cert(certificate_path, private_key_path)
                            .tls_required(true);

                        if let Some(path_to_root_certificate) = path_to_root_certificate {
                            debug!("NatsClientConfig - Trusting CA: {}", path_to_root_certificate);
                            options = options.add_root_certificate(path_to_root_certificate)
                        }
                    }
                    NatsClientAuth::None => {
                        info!("NatsClientConfig - Open Nats connection (without TLS) to [{}]", addresses);
                    }
                };
                self.nats_connection = match options.connect(&addresses).await {
                    Ok(connection) => Arc::new(Some(connection)),
                    Err(connection_error) => {
                        error!("Error during connection to NATS. Err: {}", connection_error);
                        time::delay_until(time::Instant::now() + time::Duration::new(1, 0)).await;
                        Arc::new(None)
                    }
                }

            }
        };
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
        if let Some(connection) = self.nats_connection.deref() {
            let event = serde_json::to_vec(&msg.event)
                .map_err(|err| TornadoCommonActorError::SerdeError { message: format! {"{}", err} })?;

            let client = connection.clone();
            let config = self.config.clone();

            actix::spawn(async move {
                debug!("NatsPublisherActor - Publish event to NATS");
                let res = client.publish(&config.subject, &event).await;
                match res {
                    Ok(_) => error!("NatsPublisherActor - Publish event to NATS succeeded"),
                    Err(e) => error!("NatsPublisherActor - Error sending event to NATS. Err: {}", e)
                }
                // if let Err(flush_res) = client.flush().await{
                //     error!("NatsPublisherActor - flush failed. Err: {}", flush_res);
                // }
            });
        }

        Ok(())
    }
}
