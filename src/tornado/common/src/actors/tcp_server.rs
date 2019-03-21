use crate::TornadoError;
use actix::prelude::*;
use futures::Stream;
use log::*;
use std::net;
use std::str::FromStr;
use tokio_tcp::{TcpStream, TcpListener};

pub fn listen_to_tcp_port<
    P: 'static + Into<String>,
    F: 'static + FnMut(TcpConnectMessage) -> () + Sized,
>(
    address: P,
    callback: F,
) -> Result<(), TornadoError> {
    let address = address.into();
    let socket_address = net::SocketAddr::from_str(address.as_str()).unwrap();
      let listener = TcpListener::bind(&socket_address)
            .map_err(|err| TornadoError::ActorCreationError {
                message: format!("Cannot start TCP server on [{}]: {}", address, err),
            })?;

    UdsServerActor::create(|ctx| {
        ctx.add_message_stream(listener.incoming().map_err(|e| panic!("err={:?}", e)).map(
            |stream| {
                //let addr = stream.peer_addr().unwrap();
                TcpConnectMessage { stream }
            },
        ));
        UdsServerActor { address, callback }
    });

    Ok(())
}

struct UdsServerActor<F>
where
    F: 'static + FnMut(TcpConnectMessage) -> () + Sized,
{
    address: String,
    callback: F,
}

impl<F> Actor for UdsServerActor<F>
where
    F: 'static + FnMut(TcpConnectMessage) -> () + Sized,
{
    type Context = Context<Self>;
}

#[derive(Message)]
pub struct TcpConnectMessage {
    pub stream: TcpStream,
}

impl<F> Handler<TcpConnectMessage> for UdsServerActor<F>
where
    F: 'static + FnMut(TcpConnectMessage) -> () + Sized,
{
    type Result = ();

    fn handle(&mut self, msg: TcpConnectMessage, _: &mut Context<Self>) {
        info!("UdsServerActor - new client connected to [{}]", &self.address);
        (&mut self.callback)(msg);
    }
}
