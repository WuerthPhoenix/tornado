use crate::actors::message::{EventMessage, TornadoCommonActorError};
use actix::prelude::*;
use log::*;
use std::io::Error;
use std::net;
use std::str::FromStr;
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use tokio::time;
use tokio_util::codec::{LinesCodec, LinesCodecError};

pub struct TcpClientActor {
    restarted: bool,
    address: String,
    tx: Option<actix::io::FramedWrite<String, WriteHalf<TcpStream>, LinesCodec>>,
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

        let mut sleep_until = time::Instant::now();
        if self.restarted {
            sleep_until += time::Duration::new(1, 0)
        }

        ctx.wait(
            async move {
                time::sleep_until(sleep_until).await;
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
                    warn!("TCP connection failed. Err: {:?}", err);
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
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, msg: EventMessage, ctx: &mut Context<Self>) -> Self::Result {
        let trace_id = msg.event.trace_id;
        let _span = tracing::error_span!("TcpClientActor", trace_id).entered();
        trace!("TcpClientActor - Handling Event to be sent through TCP - {:?}", &msg.event);

        match &mut self.tx {
            Some(stream) => {
                let event = serde_json::to_string(&msg.event).map_err(|err| {
                    TornadoCommonActorError::SerdeError { message: format! {"{}", err} }
                })?;
                debug!("TcpClientActor - Publishing event");
                stream.write(event);
                Ok(())
            }
            None => {
                warn!("TCP connection not available");
                ctx.address().try_send(msg).unwrap_or_else(|err| {
                    error!(
                        "TcpClientActor -  Error while sending EventMessage to itself. Error: {}",
                        err
                    )
                });
                ctx.stop();
                Err(TornadoCommonActorError::ServerNotAvailableError {
                    address: self.address.clone(),
                })
            }
        }
    }
}
