use std::sync::Arc;

use tokio::{
    io::{BufReader, ReadHalf},
    net::TcpStream,
    sync::{mpsc::Sender, Mutex},
};
use tokio_rustls::client::TlsStream;

/// A Message stream is a stream which reads a NetstringStream from a tls connection.
pub type MessageStream = BufReader<TlsStream<TcpStream>>;
pub type Writer = Sender<Action>;
pub type Reader = Arc<Mutex<ReadHalf<MessageStream>>>;

/// The default port for icinga2 API clients is 5665
#[allow(dead_code)]
pub const DEFAULT_API_PORT: u16 = 5665;

/// Defines the action for the sender function.
pub enum Action {
    Send(Vec<u8>),
    Flush,
    Shutdown,
}
