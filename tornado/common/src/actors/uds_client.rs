use actix::prelude::*;
use failure_derive::Fail;
use log::*;
use serde_json;
use std::io::Error;
use std::path::PathBuf;
use tokio::io::WriteHalf;
use tokio::net::UnixStream;
use tokio::time;
use tokio_util::codec::{LinesCodec, LinesCodecError};
use tornado_common_api;

pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

impl Message for EventMessage {
    type Result = Result<(), UdsClientActorError>;
}

#[derive(Fail, Debug)]
pub enum UdsClientActorError {
    #[fail(display = "UdsSocketNotAvailable: cannot connect to [{:?}]", socket)]
    UdsSocketNotAvailableError { socket: PathBuf },
    #[fail(display = "SerdeError: [{}]", message)]
    SerdeError { message: String },
}

pub struct UdsClientActor {
    restarted: bool,
    socket_path: PathBuf,
    tx: Option<actix::io::FramedWrite<WriteHalf<UnixStream>, LinesCodec>>,
}

impl actix::io::WriteHandler<Error> for UdsClientActor {}

impl UdsClientActor {
    pub fn start_new<T: Into<PathBuf> + 'static>(
        socket_path: T,
        uds_socket_mailbox_capacity: usize,
    ) -> Addr<UdsClientActor> {
        actix::Supervisor::start(move |ctx: &mut Context<UdsClientActor>| {
            ctx.set_mailbox_capacity(uds_socket_mailbox_capacity);
            UdsClientActor { restarted: false, socket_path: socket_path.into(), tx: None }
        })
    }
}

impl Actor for UdsClientActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("UdsClientActor started. Attempt connection to socket [{:?}]", &self.socket_path);

        let mut delay_until = time::Instant::now();
        if self.restarted {
            delay_until += time::Duration::new(1, 0)
        }
        let path = (&self.socket_path).clone();

        ctx.wait(
            async move {
                time::delay_until(delay_until).await;
                UnixStream::connect(path).await.map_err(|_| ())
            }
            .into_actor(self)
            .map(move |stream, act, ctx| match stream {
                Ok(stream) => {
                    info!("UdsClientActor connected to socket [{:?}]", &act.socket_path);
                    let (_r, w) = tokio::io::split(stream);
                    act.tx = Some(actix::io::FramedWrite::new(w, LinesCodec::new(), ctx));
                }
                Err(_) => {
                    warn!("UDS connection failed");
                    ctx.stop();
                }
            }),
        );
        /*
        tokio::timer::Delay::new(delay_until)
            .map_err(|_| ())
            .and_then(move |_| tokio_uds::UnixStream::connect(path).map_err(|_| ()))
            .into_actor(self)
            .map(move |stream, act, ctx| {
                info!("UdsClientActor connected to socket [{:?}]", &act.socket_path);
                let (_r, w) = stream.split();
                act.tx = Some(actix::io::FramedWrite::new(w, LinesCodec::new(), ctx));
            })
            .map_err(|err, act, ctx| {
                warn!(
                    "UdsClientActor failed to connected to socket [{:?}]: {:?}",
                    &act.socket_path, err
                );
                ctx.stop();
            })
            .wait(ctx);
            */
    }
}

impl actix::Supervised for UdsClientActor {
    fn restarting(&mut self, _ctx: &mut Context<UdsClientActor>) {
        info!("Restarting UdsClientActor");
        self.restarted = true;
    }
}

impl actix::io::WriteHandler<LinesCodecError> for UdsClientActor {}

impl Handler<EventMessage> for UdsClientActor {
    type Result = Result<(), UdsClientActorError>;

    fn handle(&mut self, msg: EventMessage, ctx: &mut Context<Self>) -> Self::Result {
        trace!("UdsClientActor - {:?} - received new event", &msg.event);

        match &mut self.tx {
            Some(stream) => {
                let event = serde_json::to_string(&msg.event).map_err(|err| {
                    UdsClientActorError::SerdeError { message: format! {"{}", err} }
                })?;
                stream.write(event);
                Ok(())
            }
            None => {
                warn!("Uds connection not available");
                ctx.stop();
                Err(UdsClientActorError::UdsSocketNotAvailableError {
                    socket: self.socket_path.clone(),
                })
            }
        }
    }
}
