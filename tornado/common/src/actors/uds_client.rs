use crate::actors::message::{EventMessage, TornadoCommonActorError};
use actix::prelude::*;
use log::*;
use std::io::Error;
use std::path::PathBuf;
use tokio::io::WriteHalf;
use tokio::net::UnixStream;
use tokio::time;
use tokio_util::codec::{LinesCodec, LinesCodecError};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub struct UdsClientActor {
    restarted: bool,
    socket_path: PathBuf,
    tx: Option<actix::io::FramedWrite<String, WriteHalf<UnixStream>, LinesCodec>>,
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

        let mut sleep_until = time::Instant::now();
        if self.restarted {
            sleep_until += time::Duration::new(1, 0)
        }
        let path = self.socket_path.clone();

        ctx.wait(
            async move {
                time::sleep_until(sleep_until).await;
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
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, msg: EventMessage, ctx: &mut Context<Self>) -> Self::Result {
        let parent_span = msg.0.span.clone().entered();
        let trace_id = msg.0.event.get_trace_id_for_logging(&parent_span.context());
        let _span = tracing::error_span!("UdsClientActor", trace_id = trace_id.as_ref()).entered();

        trace!("UdsClientActor - Handling Event to be sent through UDS - {:?}", &msg.0.event);

        match &mut self.tx {
            Some(stream) => {
                let event = serde_json::to_string(&msg.0.event).map_err(|err| {
                    TornadoCommonActorError::SerdeError { message: format! {"{}", err} }
                })?;
                debug!("UdsClientActor - Publishing event");
                stream.write(event);
                Ok(())
            }
            None => {
                warn!("Uds connection not available");
                ctx.stop();
                Err(TornadoCommonActorError::ServerNotAvailableError {
                    address: format!("{:?}", self.socket_path),
                })
            }
        }
    }
}
