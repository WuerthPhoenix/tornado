use crate::actors::message::{EventMessage, TornadoCommonActorError};
use crate::TornadoError;
use actix::prelude::*;
use async_nats::{Connection, Options};
use log::*;
use opentelemetry::metrics::{Counter, Meter};
use opentelemetry::trace::SpanKind;
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::ops::Deref;
use std::rc::Rc;
use tokio::time;
use tornado_common_logger::opentelemetry_logger::TelemetryContextInjector;
use tornado_common_metrics::opentelemetry::sdk::propagation::TraceContextPropagator;
use tracing_futures::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

const WAIT_BETWEEN_RESTARTS_SEC: u64 = 10;

pub struct NatsPublisherActor {
    config: NatsPublisherConfig,
    nats_connection: Rc<Option<Connection>>,
    restarted: bool,
    trace_context_propagator: TraceContextPropagator,
    metrics: Option<NatsMetrics>,
}

struct NatsMetrics {
    bytes_sent: Counter<u64>,
    send_failed: Counter<u64>,
    reconnect_attempts: Counter<u64>,
}

impl NatsMetrics {
    fn new(meter: &Meter) -> Self {
        NatsMetrics {
            bytes_sent: meter.u64_counter("nats_bytes_sent").init(),
            send_failed: meter.u64_counter("nats_send_failed").init(),
            reconnect_attempts: meter.u64_counter("nats_reconnect_attempts").init(),
        }
    }
}

impl actix::io::WriteHandler<Error> for NatsPublisherActor {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NatsPublisherConfig {
    pub client: NatsClientConfig,
    pub subject: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum NatsClientAuth {
    None,
    Tls {
        certificate_path: String,
        private_key_path: String,
        path_to_root_certificate: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NatsClientConfig {
    pub addresses: Vec<String>,
    pub auth: Option<NatsClientAuth>,
}

impl NatsClientConfig {
    pub async fn new_client(&self) -> std::io::Result<Connection> {
        let addresses = self.addresses.join(",");

        let auth = self.get_auth();

        let mut options = Options::new()
            .disconnect_callback(|| error!("NatsClientConfig - connection to NATS server was lost"))
            .reconnect_callback(|| {
                info!("NatsClientConfig - connection to NATS server was restored")
            })
            .max_reconnects(None);
        match auth {
            NatsClientAuth::Tls {
                certificate_path,
                private_key_path,
                path_to_root_certificate,
            } => {
                info!("NatsClientConfig - Open Nats connection (with TLS) to [{}]", addresses);
                options =
                    options.client_cert(certificate_path, private_key_path).tls_required(true);

                if let Some(path_to_root_certificate) = path_to_root_certificate {
                    debug!("NatsClientConfig - Trusting CA: {}", path_to_root_certificate);
                    options = options.add_root_certificate(path_to_root_certificate)
                }
            }
            NatsClientAuth::None => {
                info!("NatsClientConfig - Open Nats connection (without TLS) to [{}]", addresses);
            }
        };
        options.connect(&addresses).await
    }

    fn get_auth(&self) -> &NatsClientAuth {
        match &self.auth {
            None => &NatsClientAuth::None,
            Some(auth) => auth,
        }
    }
}

impl NatsPublisherActor {
    pub async fn start_new(
        config: NatsPublisherConfig,
        message_mailbox_capacity: usize,
    ) -> Result<Addr<NatsPublisherActor>, TornadoError> {
        Self::new(config).start(message_mailbox_capacity).await
    }

    pub fn new(config: NatsPublisherConfig) -> Self {
        let trace_context_propagator = TraceContextPropagator::new();
        NatsPublisherActor {
            config,
            nats_connection: Rc::new(None),
            restarted: false,
            trace_context_propagator,
            metrics: None,
        }
    }

    pub async fn start(
        self,
        message_mailbox_capacity: usize,
    ) -> Result<Addr<NatsPublisherActor>, TornadoError> {
        Ok(actix::Supervisor::start(move |ctx: &mut Context<NatsPublisherActor>| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            self
        }))
    }

    pub fn enable_metrics(mut self, meter: &Meter) -> Self {
        self.metrics = Some(NatsMetrics::new(meter));
        self
    }
}

impl Actor for NatsPublisherActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(
            "NatsPublisherActor started. Connecting to NATS address(es): {:?}",
            self.config.client.addresses
        );

        let client_config = self.config.client.clone();
        let nats_connection = self.nats_connection.clone();
        let restarted = self.restarted;
        if let Some(metrics) = &self.metrics {
            metrics.reconnect_attempts.add(1, &[]);
        }
        ctx.wait(
            async move {
                if restarted {
                    info!(
                        "NatsPublisherActor was restarted after a failure. Waiting {} seconds before proceeding ...",
                        WAIT_BETWEEN_RESTARTS_SEC
                    );
                    time::sleep(time::Duration::from_secs(WAIT_BETWEEN_RESTARTS_SEC)).await;
                }
                if let Some(connection) = nats_connection.deref() {
                    connection.close().await.unwrap();
                    match connection.close().await {
                        Ok(()) => {debug!(
                            "NatsPublisherActor - Successfully closed previously opened NATS connection."
                        );}
                        Err(err) => {
                            error!("NatsPublisherActor - Error while closing previously opened NATS connection. Err: {:?}", err)
                        }
                    };
                }
                client_config.new_client().await
            }
            .into_actor(self)
                .map(move |client, act, ctx| match client {
                    Ok(client) => {
                        info!(
                            "NatsPublisherActor connected to server [{:?}]",
                            &act.config.client.addresses
                        );
                        act.nats_connection = Rc::new(Some(client));
                    }
                    Err(err) => {
                        act.nats_connection = Rc::new(None);
                        warn!("NatsPublisherActor connection failed. Err: {}", err);
                        ctx.stop();
                    }
                }),
        );
    }
}

impl actix::Supervised for NatsPublisherActor {
    fn restarting(&mut self, _ctx: &mut Context<NatsPublisherActor>) {
        info!("Restarting NatsPublisherActor");
        self.restarted = true;
    }
}

impl Handler<EventMessage> for NatsPublisherActor {
    type Result = Result<(), TornadoCommonActorError>;

    fn handle(&mut self, mut msg: EventMessage, ctx: &mut Context<Self>) -> Self::Result {
        let _parent_span = msg.0.span.clone().entered();
        // Hardcode the service.name of the receiver. Currenlty publishers only publish to tornado.
        // Implementing the logic to have this not hardcoded is not worth the effort atm.
        let span = tracing::info_span!("Send Event to NATS", trace_id = tracing::field::Empty,
            otel.name = format!("Send Event to NATS subject: {}", &self.config.subject).as_str(),
            otel.kind = %SpanKind::Producer,
            peer.service = "tornado")
        .entered();
        let trace_id = msg.0.event.get_trace_id_for_logging(&span.context());
        span.record("trace_id", trace_id.as_ref());
        let trace_context = TelemetryContextInjector::get_trace_context_map(
            &span.context(),
            &self.trace_context_propagator,
        );
        msg.0.event.set_trace_context(trace_context);

        trace!("NatsPublisherActor - Handling Event to be sent to Nats - {:?}", &msg.0.event);

        let address = ctx.address();

        if let Some(connection) = self.nats_connection.deref() {
            let event = serde_json::to_vec(&msg.0.event).map_err(|err| {
                TornadoCommonActorError::SerdeError { message: format! {"{}", err} }
            })?;
            let event_len = event.len();

            let client = connection.clone();
            let config = self.config.clone();

            ctx.wait(
                async move {
                    debug!("NatsPublisherActor - Publishing event to NATS");
                    match client.publish(&config.subject, &event).await {
                        Ok(_) => trace!(
                            "NatsPublisherActor - Publish event to NATS succeeded. Event: {:?}",
                            &msg
                        ),
                        Err(e) => {
                            error!("NatsPublisherActor - Error sending event to NATS. Err: {:?}", e);
                            time::sleep(time::Duration::from_secs(1)).await;
                            address.try_send(msg).unwrap_or_else(|err| error!("NatsPublisherActor -  Error while sending event to itself. Error: {}", err));
                        }
                    }
                }
                .instrument(span.exit())
                .into_actor(self)
            );
            if let Some(metrics) = &self.metrics {
                metrics.bytes_sent.add(event_len as u64, &[]);
            }
        } else {
            if let Some(metrics) = &self.metrics {
                metrics.send_failed.add(1, &[]);
            }
            warn!("NatsPublisherActor - Processing event but NATS connection not yet established. Stopping actor and reprocessing the event ...");
            ctx.stop();
            address.try_send(msg).unwrap_or_else(|err| {
                error!("NatsPublisherActor -  Error while sending event to itself. Err: {:?}", err)
            });
        }

        Ok(())
    }
}

pub async fn wait_for_nats_connection(client_config: &NatsClientConfig) -> Connection {
    loop {
        match client_config.new_client().await {
            Err(connection_error) => {
                error!("Error during connection to NATS. Err: {:?}", connection_error);
                time::sleep(time::Duration::from_secs(5)).await;
            }
            Ok(connection) => {
                info!("NatsClientConfig - Successfully connected to NATS");
                return connection;
            }
        }
    }
}
