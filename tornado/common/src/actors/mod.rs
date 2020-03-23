#[cfg(feature = "nats")]
use crate::actors::nats_publisher::NatsPublisherConfig;
use serde_derive::{Deserialize, Serialize};

pub mod json_event_reader;
pub mod message;
pub mod tcp_client;
pub mod tcp_server;

#[cfg(feature = "nats")]
pub mod nats_publisher;
#[cfg(feature = "nats")]
pub mod nats_subscriber;

#[cfg(unix)]
pub mod uds_client;
#[cfg(unix)]
pub mod uds_server;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum TornadoConnectionChannel {
    #[cfg(feature = "nats")]
    NatsStreaming {
        nats_streaming: NatsPublisherConfig,
    },
    TCP {
        tcp_socket_ip: String,
        tcp_socket_port: u16,
    },
}
