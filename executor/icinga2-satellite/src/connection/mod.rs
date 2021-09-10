use std::sync::Arc;
use std::time::Duration;

use log::{debug, warn};
use tokio::{
    io::{AsyncWriteExt, Error, ErrorKind, Result, WriteHalf},
    sync::{
        mpsc::{channel, Receiver},
        Mutex,
    },
    task::JoinHandle,
};

use super::message::*;
use self::types::*;
use tornado_common_netstring::{NetstringWriter, NetstringReader};

mod types;

const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024;

/// Represents an established connection to another icinga2 instance. It contains a [Sender](tokio::sync::mpsc::Sender)
/// which gives the user access to the Stream to the endpoint a [WriteHalf](tokio::io::WriteHalf)
/// for incoming messages and a JoinHandle for the Sender task.
pub struct Connection {
    buf: Vec<u8>,
    reader: Reader,
    writer: Writer,
    handler: JoinHandle<()>,
}

impl Connection {
    /// creates an icinga2 Connection from a [MessageStream](netstring::NetstringStream)
    pub(crate) async fn from(mut stream: MessageStream) -> Result<Connection> {
        Self::send_hello(&mut stream).await?;

        let (reader, writer) = tokio::io::split(stream);
        let (tx, rx) = channel(1024 * 64); //todo: make this configurable
        let tx2 = tx.clone();

        // will kill itself once the connection thread is dropped
        tokio::task::spawn(async move { Connection::heartbeat(tx2).await });

        Ok(Connection {
            buf: vec![0; DEFAULT_BUFFER_SIZE], //todo: make this configurable
            reader: Arc::new(Mutex::new(reader)),
            writer: tx,
            handler: tokio::task::spawn(async { Self::sender(rx, writer).await }),
        })
    }

    async fn send_hello(writer: &mut MessageStream) -> Result<()> {
        const ICINGA_HELLO: JsonRpc = JsonRpc::JsonRpc2 {
            message: Message::Hello(EmptyParams {}),
            ts: None,
        };
        let msg = serde_json::to_vec(&ICINGA_HELLO).expect("Is constant and will always serialize");

        writer.write_netstring(&msg).await.map(|_| ())
    }

    async fn heartbeat(writer: Writer) {
        const HEARTBEAT: JsonRpc = JsonRpc::JsonRpc2 {
            message: Message::HeartBeats(EmptyParams {}),
            ts: None,
        };
        let msg = serde_json::to_vec(&HEARTBEAT).expect("Is constant and will always serialize");
        tokio::time::sleep(Duration::from_secs(20)).await;

        loop {
            let res1 = writer.send(Action::Send(msg.clone())).await;
            let res2 = writer.send(Action::Flush).await;

            match res1.is_ok() && res2.is_ok() {
                true => tokio::time::sleep(Duration::from_secs(20)).await,
                false => break
            }
        }

        warn!("Icinga2ConnectionHeartbeat - Could not continue sending the heartbeat. Dropping task");
    }

    // todo: Maybe make an actual future out of this for better readability?
    async fn sender(mut rx: Receiver<Action>, mut writer: WriteHalf<MessageStream>) {
        while let Some(action) = rx.recv().await {
            match action {
                Action::Send(msg) => {
                    debug!(">> {}", String::from_utf8_lossy(&msg));
                    writer.write_netstring(&msg).await.map(|_| ())
                }
                Action::Flush => writer.flush().await,
                Action::Shutdown => {
                    // give the master time to process all messages before closing the connection
                    writer.shutdown().await.expect("Connection lost");
                    return;
                }
            }
                .expect("Connection lost");
        }
    }

    /// send a [message](icinga2::message::Message) to the connected endpoint
    pub(crate) async fn send(&self, msg: JsonRpc) -> Result<()> {
        let msg = serde_json::to_vec(&msg)
            .map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))?;

        self.writer.send(Action::Send(msg)).await.map_err(|_| {
            Error::new(
                ErrorKind::BrokenPipe,
                "Connection closed and receiver dropped",
            )
        })
    }

    /// flushes everything in the current queue
    pub async fn flush(&self) -> Result<()> {
        self.writer.send(Action::Flush).await.map_err(|_| {
            Error::new(
                ErrorKind::BrokenPipe,
                "Connection closed and receiver dropped",
            )
        })
    }

    /// receive a [message](icinga2::message::Message) from the connected endpoint. Returns
    /// an Error if the current message is not known or the connection has been dropped. This
    /// can be due to a malformed json, or because the library doesn't currently support the message.
    pub async fn read_message(&mut self) -> Result<Message> {
        let buf = self.buf.as_mut_slice();

        let size = self.reader.lock().await.read_netstring(buf).await?;
        let msg: JsonRpc = serde_json::from_slice(&buf[..size])
            .map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))?;

        debug!("<< {}", String::from_utf8_lossy(&buf[..size]));

        match msg {
            JsonRpc::JsonRpc2 { message, .. } => Ok(message),
        }
    }

    /// closes the connection.
    pub async fn shutdown(self) -> Result<()> {
        let res = self
            .writer
            .send(Action::Shutdown)
            .await
            .map_err(|_| ErrorKind::BrokenPipe.into());

        debug!("Icinga2Connection - Shutdown action was sent. Awaiting the task to finish processing the queue.");

        self.handler.await?;
        res
    }

    pub async fn handle(self) -> JoinHandle<()> {
        self.handler
    }
}
