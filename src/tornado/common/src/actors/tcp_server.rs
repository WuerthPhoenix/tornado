use crate::actors::message::AsyncReadMessage;
use crate::TornadoError;
use actix::prelude::*;
use futures::Stream;
use log::*;
use std::net;
use std::str::FromStr;
use tokio_tcp::{TcpListener, TcpStream};

pub fn listen_to_tcp<
    P: 'static + Into<String>,
    F: 'static + FnMut(AsyncReadMessage<TcpStream>) -> () + Sized,
>(
    address: P,
    callback: F,
) -> Result<(), TornadoError> {
    let address = address.into();
    let socket_address = net::SocketAddr::from_str(address.as_str()).unwrap();
    let listener =
        TcpListener::bind(&socket_address).map_err(|err| TornadoError::ActorCreationError {
            message: format!("Cannot start TCP server on [{}]: {}", address, err),
        })?;

    TcpServerActor::create(|ctx| {
        ctx.add_message_stream(listener.incoming().map_err(|e| panic!("err={:?}", e)).map(
            |stream| {
                //let addr = stream.peer_addr().unwrap();
                AsyncReadMessage { stream }
            },
        ));
        TcpServerActor { address, callback }
    });

    Ok(())
}

struct TcpServerActor<F>
where
    F: 'static + FnMut(AsyncReadMessage<TcpStream>) -> () + Sized,
{
    address: String,
    callback: F,
}

impl<F> Actor for TcpServerActor<F>
where
    F: 'static + FnMut(AsyncReadMessage<TcpStream>) -> () + Sized,
{
    type Context = Context<Self>;
}

impl<F> Handler<AsyncReadMessage<TcpStream>> for TcpServerActor<F>
where
    F: 'static + FnMut(AsyncReadMessage<TcpStream>) -> () + Sized,
{
    type Result = ();

    fn handle(&mut self, msg: AsyncReadMessage<TcpStream>, _: &mut Context<Self>) {
        info!("TcpServerActor - new client connected to [{}]", &self.address);
        (&mut self.callback)(msg);
    }
}
