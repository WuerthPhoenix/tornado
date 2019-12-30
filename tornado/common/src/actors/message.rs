use actix::prelude::Message;
use tokio::prelude::AsyncRead;

#[derive(Message)]
#[rtype(result="()")]
pub struct StringMessage {
    pub msg: String,
}

#[derive(Message)]
#[rtype(result="()")]
pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

#[derive(Message)]
#[rtype(result="()")]
pub struct AsyncReadMessage<R: AsyncRead> {
    pub stream: R,
}
