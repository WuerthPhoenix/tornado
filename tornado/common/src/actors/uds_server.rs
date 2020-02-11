use crate::actors::message::AsyncReadMessage;
use crate::TornadoError;
use actix::prelude::*;
use futures::StreamExt;
use log::*;
use std::fs;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use tokio::net::{UnixListener, UnixStream};

pub fn listen_to_uds_socket<
    P: 'static + Into<String>,
    F: 'static + FnMut(AsyncReadMessage<UnixStream>) -> () + Sized + Unpin,
>(
    path: P,
    socket_permissions: Option<u32>,
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

    UdsServerActor::create(move |ctx| {
        ctx.add_message_stream(Box::leak(Box::new(listener)).incoming().map(|stream| {
            //let addr = stream.peer_addr().unwrap();
            AsyncReadMessage { stream: stream.expect("Cannot read from UDS server stream") }
        }));
        UdsServerActor { path: path_string, socket_permissions, callback }
    });

    Ok(())
}

struct UdsServerActor<F>
where
    F: 'static + FnMut(AsyncReadMessage<UnixStream>) -> () + Sized + Unpin,
{
    path: String,
    socket_permissions: Option<u32>,
    callback: F,
}

impl<F> Actor for UdsServerActor<F>
where
    F: 'static + FnMut(AsyncReadMessage<UnixStream>) -> () + Sized + Unpin,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if let Some(permissions) = self.socket_permissions {
            debug!("UdsServerActor - Set filesystem socket permissions to [{:o}]", permissions);
            if let Err(err) = fs::set_permissions(&self.path, Permissions::from_mode(permissions)) {
                error!("UdsServerActor - Cannot set socket permissions. Err: {}", err);
                ctx.stop();
            }
        }
    }
}

/// Handle a stream of UnixStream elements
impl<F> Handler<AsyncReadMessage<UnixStream>> for UdsServerActor<F>
where
    F: 'static + FnMut(AsyncReadMessage<UnixStream>) -> () + Sized + Unpin,
{
    type Result = ();

    fn handle(&mut self, msg: AsyncReadMessage<UnixStream>, _: &mut Context<Self>) {
        debug!("UdsServerActor - new client connected to [{}]", &self.path);
        (&mut self.callback)(msg);
    }
}
