use crate::TornadoError;
use actix::prelude::*;
use futures::Stream;
use log::*;
use std::fs;
use tokio_uds::*;

pub fn listen_to_uds_socket<
    P: Into<String>,
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
>(
    path: P,
    callback: F,
) -> Result<(), TornadoError> {
    let path_string = path.into();
    let listener = match UnixListener::bind(&path_string) {
        Ok(m) => m,
        Err(_) => {
            fs::remove_file(&path_string).map_err(|err| TornadoError::ActorCreationError {
                message: format!(
                    "Cannot bind UDS socket to path [{}] and cannot remove such file if exists: {}",
                    path_string, err
                ),
            })?;
            UnixListener::bind(&path_string).map_err(|err| TornadoError::ActorCreationError {
                message: format!("Cannot bind UDS socket to path [{}]: {}", path_string, err),
            })?
        }
    };

    UdsServerActor::create(|ctx| {
        ctx.add_message_stream(listener.incoming().map_err(|e| panic!("err={:?}", e)).map(
            |stream| {
                //let addr = stream.peer_addr().unwrap();
                UdsConnectMessage { stream }
            },
        ));
        UdsServerActor { path: path_string, callback }
    });

    Ok(())
}

struct UdsServerActor<F>
where
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
{
    path: String,
    callback: F,
}

impl<F> Actor for UdsServerActor<F>
where
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
{
    type Context = Context<Self>;
}

#[derive(Message)]
pub struct UdsConnectMessage {
    pub stream: UnixStream,
}

/// Handle a stream of UnixStream elements
impl<F> Handler<UdsConnectMessage> for UdsServerActor<F>
where
    F: 'static + FnMut(UdsConnectMessage) -> () + Sized,
{
    type Result = ();

    fn handle(&mut self, msg: UdsConnectMessage, _: &mut Context<Self>) {
        info!("UdsServerActor - new client connected to [{}]", &self.path);
        (&mut self.callback)(msg);
    }
}
