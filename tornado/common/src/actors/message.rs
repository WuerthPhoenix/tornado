use actix::prelude::Message;
use std::sync::Arc;
use thiserror::Error;
use tokio::prelude::AsyncRead;
use tornado_common_api::Action;
use tornado_executor_common::ExecutorError;

#[derive(Debug, Message, Clone)]
#[rtype(result = "Result<(), ExecutorError>")]
pub struct ActionMessage {
    pub action: Arc<Action>,
}

#[derive(Error, Debug)]
pub enum TornadoCommonActorError {
    #[error("ServerNotAvailableError: cannot connect to server [{address}]")]
    ServerNotAvailableError { address: String },
    #[error("SerdeError: [{message}]")]
    SerdeError { message: String },
    #[error("GenericError: [{message}]")]
    GenericError { message: String },
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct StringMessage {
    pub msg: String,
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), TornadoCommonActorError>")]
pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

#[derive(Message)]
#[rtype(result = "Result<(), TornadoCommonActorError>")]
pub struct ResetActorMessage<P> {
    pub payload: P,
}

#[derive(Message, Debug)]
#[rtype(result = "Result<(), TornadoCommonActorError>")]
pub struct BytesMessage {
    pub msg: Vec<u8>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AsyncReadMessage<R: AsyncRead> {
    pub stream: R,
}
