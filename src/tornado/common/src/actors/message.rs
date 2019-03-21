use actix::prelude::Message;
use tokio::prelude::AsyncRead;

#[derive(Message)]
pub struct StringMessage {
    pub msg: String,
}

#[derive(Message)]
pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

#[derive(Message)]
pub struct AsyncReadMessage<R: AsyncRead> {
    pub stream: R,
}