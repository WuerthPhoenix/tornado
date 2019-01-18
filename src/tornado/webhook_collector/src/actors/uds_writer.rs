//
// ToDo: code copied from rsyslog_collector. To be moved in common library.
//

use actix::prelude::*;
use failure_derive::Fail;
use log::*;
use serde_json;
use std::io::Error;
use std::path::PathBuf;
use std::time;
use tokio::io::WriteHalf;
use tokio::prelude::*;
use tokio_codec::LinesCodec;
use tokio_uds::*;
use tornado_common_api;

pub struct EventMessage {
    pub event: tornado_common_api::Event,
}

impl Message for EventMessage {
    type Result = Result<(), UdsWriterActorError>;
}

#[derive(Fail, Debug)]
pub enum UdsWriterActorError {
    #[fail(display = "UdsSocketNotAvailable: cannot connect to [{}]", socket)]
    UdsSocketNotAvailableError { socket: String },
    #[fail(display = "SerdeError: [{}]", message)]
    SerdeError { message: String },
}

pub struct UdsWriterActor {
    restarted: bool,
    socket_path: PathBuf,
    tx: Option<actix::io::FramedWrite<WriteHalf<UnixStream>, LinesCodec>>,
}

impl actix::io::WriteHandler<Error> for UdsWriterActor {}

impl UdsWriterActor {
    pub fn start_new<T: Into<PathBuf> + 'static>(
        socket_path: T,
        uds_socket_mailbox_capacity: usize,
    ) -> Addr<UdsWriterActor> {
        actix::Supervisor::start(move |ctx: &mut Context<UdsWriterActor>| {
            ctx.set_mailbox_capacity(uds_socket_mailbox_capacity);
            UdsWriterActor { restarted: false, socket_path: socket_path.into(), tx: None }
        })
    }
}

impl Actor for UdsWriterActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("UdsWriterActor started. Attempt connection to socket [{:?}]", &self.socket_path);

        let mut delay_until = time::Instant::now();
        if self.restarted {
            delay_until += time::Duration::new(1, 0)
        }
        let path = (&self.socket_path).clone();

        tokio::timer::Delay::new(delay_until)
            .map_err(|_| ())
            .and_then(move |_| tokio_uds::UnixStream::connect(path).map_err(|_| ()))
            .into_actor(self)
            .map(move |stream, act, ctx| {
                info!("UdsWriterActor connected to socket [{:?}]", &act.socket_path);
                let (_r, w) = stream.split();
                act.tx = Some(actix::io::FramedWrite::new(w, LinesCodec::new(), ctx));
            })
            .map_err(|err, act, ctx| {
                warn!(
                    "UdsWriterActor failed to connected to socket [{:?}]: {:?}",
                    &act.socket_path, err
                );
                ctx.stop();
            })
            .wait(ctx);
    }
}

impl actix::Supervised for UdsWriterActor {
    fn restarting(&mut self, _ctx: &mut Context<UdsWriterActor>) {
        info!("Restarting UdsWriterActor");
        self.restarted = true;
    }
}

impl Handler<EventMessage> for UdsWriterActor {
    type Result = Result<(), UdsWriterActorError>;

    fn handle(&mut self, msg: EventMessage, ctx: &mut Context<Self>) -> Self::Result {
        trace!("UdsWriterActor - {:?} - received new event", &msg.event);

        match &mut self.tx {
            Some(stream) => {
                let event = serde_json::to_string(&msg.event).map_err(|err| {
                    UdsWriterActorError::SerdeError { message: format! {"{}", err} }
                })?;
                stream.write(event);
                Ok(())
            }
            None => {
                warn!("Uds connection not available");
                ctx.stop();
                Err(UdsWriterActorError::UdsSocketNotAvailableError { socket: "".to_owned() })
            }
        }
    }
}
