use actix::prelude::*;
use serde_json;
use std::io::Error;
use std::path::PathBuf;
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
            UdsWriterActor { socket_path: socket_path.into(), tx: None }
        })
    }
}

impl Actor for UdsWriterActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("UdsWriterActor started. Attempt connection to socket [{:?}]", &self.socket_path);

        tokio_uds::UnixStream::connect(&self.socket_path)
            .into_actor(self)
            .map(move |stream, act, ctx| {
                println!("UdsWriterActor connected to socket [{:?}]", &act.socket_path);
                let (_r, w) = stream.split();
                act.tx = Some(actix::io::FramedWrite::new(w, LinesCodec::new(), ctx));
            }).map_err(|err, act, ctx| {
                println!(
                    "UdsWriterActor failed to connected to socket [{:?}]: {}",
                    &act.socket_path, err
                );
                ctx.stop();
            }).wait(ctx);
    }
}

impl actix::Supervised for UdsWriterActor {
    fn restarting(&mut self, _ctx: &mut Context<UdsWriterActor>) {
        info!("Restarting UdsWriterActor");
    }
}

impl Handler<EventMessage> for UdsWriterActor {
    type Result = Result<(), UdsWriterActorError>;

    fn handle(&mut self, msg: EventMessage, ctx: &mut Context<Self>) -> Self::Result {
        trace!("UdsWriterActor - {:?} - received new event", &msg.event);

        match &mut self.tx {
            Some(stream) => {
                let mut event = serde_json::to_string(&msg.event).map_err(|err| {
                    UdsWriterActorError::SerdeError { message: format!{"{}", err} }
                })?;
                event.push('\n');

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
