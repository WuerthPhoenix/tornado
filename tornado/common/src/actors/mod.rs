use crate::TornadoError;
use failure::_core::str::FromStr;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize, PartialEq, Copy, Clone)]
pub enum TornadoConnectionChannel {
    TCP,
    NatsStreaming,
}

impl FromStr for TornadoConnectionChannel {
    type Err = TornadoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(TornadoConnectionChannel::TCP),
            "natsstreaming" => Ok(TornadoConnectionChannel::NatsStreaming),
            _ => Err(TornadoError::ConfigurationError {
                message: format!("Unknown Connection Channel [{}]", s),
            }),
        }
    }
}
