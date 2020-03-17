#[cfg(feature = "nats_streaming")]
use crate::actors::nats_streaming_publisher::StanPublisherConfig;
use serde_derive::{Deserialize, Serialize};

pub mod json_event_reader;
pub mod message;
pub mod tcp_client;
pub mod tcp_server;

#[cfg(feature = "nats_streaming")]
pub mod nats_streaming_publisher;
#[cfg(feature = "nats_streaming")]
pub mod nats_streaming_subscriber;

#[cfg(unix)]
pub mod uds_client;
#[cfg(unix)]
pub mod uds_server;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum TornadoConnectionChannel {
    #[cfg(feature = "nats_streaming")]
    NatsStreaming {
        nats_streaming: StanPublisherConfig,
    },
    TCP {
        tcp_socket_ip: String,
        tcp_socket_port: u16,
    },
}
