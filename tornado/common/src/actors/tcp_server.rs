use crate::actors::message::AsyncReadMessage;
use crate::TornadoError;
use actix::prelude::*;
use futures_util::StreamExt;
use log::*;
use std::net;
use std::str::FromStr;
use tokio::net::{TcpListener, TcpStream};

pub async fn listen_to_tcp<
    P: 'static + Into<String>,
    F: 'static + FnMut(AsyncReadMessage<TcpStream>) + Sized + Unpin,
>(
    address: P,
    message_mailbox_capacity: usize,
    callback: F,
) -> Result<(), TornadoError> {
    let address = address.into();
    let socket_address = net::SocketAddr::from_str(address.as_str()).unwrap();
    let listener = Box::new(tokio_stream::wrappers::TcpListenerStream::new(TcpListener::bind(&socket_address).await.map_err(|err| {
        TornadoError::ActorCreationError {
            message: format!("Cannot start TCP server on [{}]: {}", address, err),
        }
    })?));

    TcpServerActor::create(|ctx| {
        ctx.set_mailbox_capacity(message_mailbox_capacity);
        ctx.add_message_stream(Box::leak(listener).map(|stream| AsyncReadMessage {
            stream: stream.expect("Cannot read from TCP server stream"),
        }));
        TcpServerActor { address, callback }
    });

    Ok(())
}

struct TcpServerActor<F>
where
    F: 'static + FnMut(AsyncReadMessage<TcpStream>) + Sized + Unpin,
{
    address: String,
    callback: F,
}

impl<F> Actor for TcpServerActor<F>
where
    F: 'static + FnMut(AsyncReadMessage<TcpStream>) + Sized + Unpin,
{
    type Context = Context<Self>;
}

impl<F> Handler<AsyncReadMessage<TcpStream>> for TcpServerActor<F>
where
    F: 'static + FnMut(AsyncReadMessage<TcpStream>) + Sized + Unpin,
{
    type Result = ();

    fn handle(&mut self, msg: AsyncReadMessage<TcpStream>, _: &mut Context<Self>) {
        debug!("TcpServerActor - new client connected to [{}]", &self.address);
        (&mut self.callback)(msg);
    }
}
