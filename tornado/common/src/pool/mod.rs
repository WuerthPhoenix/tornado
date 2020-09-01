use crate::TornadoError;

pub mod async_pool;
pub mod blocking_pool;

#[derive(Clone)]
pub struct Sender<M, R> {
    sender: async_channel::Sender<ReplyRequest<M, R>>,
}

impl<M, R> Sender<M, R> {
    pub fn new(sender: async_channel::Sender<ReplyRequest<M, R>>) -> Self {
        Self { sender }
    }

    /// Attempts to send a message into the channel.
    ///
    /// If the channel is full or closed, this method returns an error.
    pub fn try_send(&self, msg: M) -> Result<(), TornadoError> {
        self.sender.try_send(ReplyRequest { msg, responder: None }).map_err(|err| {
            TornadoError::SenderError { message: format!("Error sending message: {:?}", err) }
        })
    }

    /// Sends a message into the channel.
    ///
    /// If the channel is full, this method waits until there is space for a message.
    ///
    /// If the channel is closed, this method returns an error.
    pub async fn send(&self, msg: M) -> Result<R, TornadoError> {
        let (tx, rx) = async_channel::bounded(1);
        self.sender.send(ReplyRequest { msg, responder: Some(tx) }).await.map_err(|err| {
            TornadoError::SenderError { message: format!("Error sending message: {:?}", err) }
        })?;
        rx.recv().await.map_err(|err| TornadoError::SenderError {
            message: format!("Error receiving message response: {:?}", err),
        })
    }
}

pub struct ReplyRequest<M, R> {
    pub msg: M,
    pub responder: Option<async_channel::Sender<R>>,
}
