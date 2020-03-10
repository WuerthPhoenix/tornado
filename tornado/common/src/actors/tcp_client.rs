use actix::prelude::*;
use log::*;
use serde_json;
use std::io::Error;
use std::net;
use std::str::FromStr;
use thiserror::Error;
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use tokio::time;
use tokio_util::codec::{LinesCodec, LinesCodecError};
use tornado_common_api;

pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

impl Message for EventMessage {
    type Result = Result<(), TcpClientActorError>;
}

#[derive(Error, Debug)]
pub enum TcpClientActorError {
    #[error("ServerNotAvailableError: cannot connect to server [{address}]")]
    ServerNotAvailableError { address: String },
    #[error("SerdeError: [{message}]")]
    SerdeError { message: String },
}

pub struct TcpClientActor {
    restarted: bool,
    address: String,
    tx: Option<actix::io::FramedWrite<WriteHalf<TcpStream>, LinesCodec>>,
}

impl actix::io::WriteHandler<Error> for TcpClientActor {}

impl TcpClientActor {
    pub fn start_new<T: 'static + Into<String>>(
        address: T,
        tcp_socket_mailbox_capacity: usize,
    ) -> Addr<TcpClientActor> {
        actix::Supervisor::start(move |ctx: &mut Context<TcpClientActor>| {
            ctx.set_mailbox_capacity(tcp_socket_mailbox_capacity);
            TcpClientActor { restarted: false, address: address.into(), tx: None }
        })
    }
}

impl Actor for TcpClientActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("TcpClientActor started. Attempting connection to server [{:?}]", &self.address);
        let socket_address =
            net::SocketAddr::from_str(self.address.as_str()).expect("Not valid socket address");

        let mut delay_until = time::Instant::now();
        if self.restarted {
            delay_until += time::Duration::new(1, 0)
        }

        ctx.wait(
            async move {
                time::delay_until(delay_until).await;
                TcpStream::connect(&socket_address).await
            }
            .into_actor(self)
            .map(move |stream, act, ctx| match stream {
                Ok(stream) => {
                    info!("TcpClientActor connected to server [{:?}]", &act.address);
                    let (_r, w) = tokio::io::split(stream);
                    act.tx = Some(actix::io::FramedWrite::new(w, LinesCodec::new(), ctx));
                }
                Err(err) => {
                    warn!("TCP connection failed. Err: {}", err);
                    ctx.stop();
                }
            }),
        );
    }
}

impl actix::Supervised for TcpClientActor {
    fn restarting(&mut self, _ctx: &mut Context<TcpClientActor>) {
        info!("Restarting TcpClientActor");
        self.restarted = true;
    }
}

impl actix::io::WriteHandler<LinesCodecError> for TcpClientActor {}

impl Handler<EventMessage> for TcpClientActor {
    type Result = Result<(), TcpClientActorError>;

    fn handle(&mut self, msg: EventMessage, ctx: &mut Context<Self>) -> Self::Result {
        trace!("TcpClientActor - {:?} - received new event", &msg.event);

        match &mut self.tx {
            Some(stream) => {
                let event = serde_json::to_string(&msg.event).map_err(|err| {
                    TcpClientActorError::SerdeError { message: format! {"{}", err} }
                })?;
                stream.write(event);
                Ok(())
            }
            None => {
                warn!("TCP connection not available");
                ctx.stop();
                Err(TcpClientActorError::ServerNotAvailableError { address: self.address.clone() })
            }
        }
    }
}
