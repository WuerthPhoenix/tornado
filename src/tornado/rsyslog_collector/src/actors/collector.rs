use actix::prelude::*;
use log::*;
use tokio::io::AsyncRead;
use tokio::prelude::Stream;
use tokio_codec::{FramedRead, LinesCodec};
use tornado_collector_common::Collector;
use tornado_collector_json::JsonPayloadCollector;
use tornado_common::actors::message::StringMessage;
use tornado_common::actors::tcp_client::{EventMessage, TcpClientActor};

pub struct RsyslogCollectorActor {
    pub collector: JsonPayloadCollector,
    pub writer_addr: Addr<TcpClientActor>,
}

impl RsyslogCollectorActor {
    pub fn start_new<S>(source: S, writer_addr: Addr<TcpClientActor>)
    where
        S: AsyncRead + 'static,
    {
        RsyslogCollectorActor::create(move |ctx| {
            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LinesCodec::new();

            let framed = FramedRead::new(source, codec).map(|msg| StringMessage { msg });
            RsyslogCollectorActor::add_stream(framed, ctx);
            RsyslogCollectorActor { collector: JsonPayloadCollector::new("syslog"), writer_addr }
        });
    }
}

impl Actor for RsyslogCollectorActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("RsyslogCollectorActor started.");
    }
}

/// To use `Framed` with an actor, we have to implement the `StreamHandler` trait
impl StreamHandler<StringMessage, std::io::Error> for RsyslogCollectorActor {
    fn handle(&mut self, msg: StringMessage, _ctx: &mut Self::Context) {
        debug!("RsyslogCollectorActor - received msg: [{}]", &msg.msg);

        match self.collector.to_event(&msg.msg) {
            Ok(event) => self.writer_addr.do_send(EventMessage { event }),
            Err(e) => error!("RsyslogCollectorActor - Cannot unmarshal event from json: {}", e),
        };
    }

    // The error method intercept errors that can happen during the actor initialization phase.
    // This would have reported that the tokio::stdin() was not not able to start due to the
    // specific runtime used by Actix (See: https://github.com/actix/actix/issues/181 )
    fn error(&mut self, err: std::io::Error, _ctx: &mut Self::Context) -> actix::Running {
        error!("{}", err);
        actix::Running::Continue
    }
}
