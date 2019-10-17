use actix::prelude::*;
use failure_derive::Fail;
use log::*;
use serde_json;
use std::io::Error;
use std::net;
use std::str::FromStr;
use std::time;
use tokio::io::WriteHalf;
use tokio::prelude::*;
use tokio_codec::LinesCodec;
use tokio_tcp::TcpStream;
use tornado_common_api;

pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

impl Message for EventMessage {
    type Result = Result<(), TcpClientActorError>;
}

#[derive(Fail, Debug)]
pub enum TcpClientActorError {
    #[fail(display = "ServerNotAvailableError: cannot connect to server [{}]", address)]
    ServerNotAvailableError { address: String },
    #[fail(display = "SerdeError: [{}]", message)]
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

        let mut delay_until = time::Instant::now();
        if self.restarted {
            delay_until += time::Duration::new(1, 0)
        }

        let socket_address = net::SocketAddr::from_str(self.address.as_str()).unwrap();

        tokio::timer::Delay::new(delay_until)
            .map_err(|_| ())
            .and_then(move |_| TcpStream::connect(&socket_address).map_err(|_| ()))
            .into_actor(self)
            .map(move |stream, act, ctx| {
                info!("TcpClientActor connected to server [{:?}]", &act.address);
                let (_r, w) = stream.split();
                act.tx = Some(actix::io::FramedWrite::new(w, LinesCodec::new(), ctx));
            })
            .map_err(|err, act, ctx| {
                warn!("TcpClientActor failed to connect to server [{:?}]: {:?}", &act.address, err);
                ctx.stop();
            })
            .wait(ctx);
    }
}

impl actix::Supervised for TcpClientActor {
    fn restarting(&mut self, _ctx: &mut Context<TcpClientActor>) {
        info!("Restarting TcpClientActor");
        self.restarted = true;
    }
}

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
