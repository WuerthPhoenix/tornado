use std::sync::Arc;

use actix::{dev::ToEnvelope, Actor, Addr, Handler};
use actix_web::{
    error,
    http::{self, StatusCode},
    web::{self, Data, PayloadConfig, Query},
    HttpResponse, Responder, Scope,
};
use chrono::Local;
use log::{debug, error, info, trace};
use opentelemetry::{
    metrics::{Counter, Meter},
    KeyValue,
};
use serde::Deserialize;
use thiserror::Error;
use tornado_collector_common::{Collector, CollectorError};
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common::actors::message::EventMessage;
use tornado_common_api::TracedEvent;
use tornado_common_metrics::Metrics;
use tracing::info_span;

use crate::config::WebhookConfig;

pub fn create_app<A>(endpoints: Vec<EndpointState<A>>, meter: Arc<Metrics>) -> Scope
where
    A: Actor + Handler<EventMessage>,
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    let mut scope = web::scope("")
        .service(web::resource("/ping").route(web::get().to(pong)))
        .service(tornado_common_metrics::endpoint::actix_web::metrics_endpoints(meter));

    for endpoint in endpoints {
        let path = format!("/event/{}", endpoint.id);
        debug!("Creating endpoint: [{}]", &path);

        let new_scope = web::scope(&path)
            .app_data(PayloadConfig::default().limit(endpoint.max_payload_size))
            .app_data(Data::new(endpoint))
            .service(web::resource("").route(web::post().to(handle::<A>)));

        scope = scope.service(new_scope);
    }

    scope
}

pub fn create_endpoint_state<A: Actor>(
    configs: Vec<WebhookConfig>,
    meter: &Meter,
    addr: Addr<A>,
) -> Result<Vec<EndpointState<A>>, CollectorError> {
    let shared_metrics = SharedEndpointMetrics::new(meter);

    let mut endpoints = vec![];
    for config in configs {
        let metrics = EndpointMetrics::new(meter);
        let jmespath_collector = JMESPathEventCollector::build(config.collector_config)?;

        endpoints.push(EndpointState {
            id: config.id,
            token: config.token,
            jmespath_collector,
            actor_address: addr.clone(),
            metrics,
            shared_metrics: shared_metrics.clone(),
            max_payload_size: config.max_payload_size.0 as usize,
        })
    }

    Ok(endpoints)
}

async fn pong() -> impl Responder {
    let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
    let created_ms: String = dt.to_rfc3339();
    format!("pong - {}", created_ms)
}

pub struct EndpointState<A: Actor> {
    id: String,
    token: String,
    max_payload_size: usize,
    jmespath_collector: JMESPathEventCollector,
    actor_address: Addr<A>,
    metrics: EndpointMetrics,
    shared_metrics: SharedEndpointMetrics,
}

impl<A: Actor> Clone for EndpointState<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            token: self.token.clone(),
            max_payload_size: self.max_payload_size,
            jmespath_collector: self.jmespath_collector.clone(),
            actor_address: self.actor_address.clone(),
            metrics: self.metrics.clone(),
            shared_metrics: self.shared_metrics.clone(),
        }
    }
}

#[derive(Clone)]
struct EndpointMetrics {
    webhooks_received: Counter<u64>,
    bytes_received: Counter<u64>,
    webhooks_failed: Counter<u64>,
}

impl EndpointMetrics {
    fn new(meter: &Meter) -> Self {
        Self {
            webhooks_received: meter.u64_counter("webhooks_received").init(),
            bytes_received: meter.u64_counter("bytes_received").init(),
            webhooks_failed: meter.u64_counter("webhooks_failed").init(),
        }
    }
}

#[derive(Clone)]
struct SharedEndpointMetrics {
    events_dropped: Counter<u64>,
}

impl SharedEndpointMetrics {
    fn new(meter: &Meter) -> Self {
        SharedEndpointMetrics { events_dropped: meter.u64_counter("events_dropped").init() }
    }
}

#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("The request cannot be processed: {message}")]
    CollectorError { message: String },
    #[error("NotValidToken")]
    WrongTokenError,
    #[error("QueueFull")]
    QueueFull,
}

#[derive(Deserialize)]
pub struct TokenQuery {
    pub token: String,
}

impl error::ResponseError for HandlerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            HandlerError::CollectorError { .. } => {
                HttpResponse::new(http::StatusCode::INTERNAL_SERVER_ERROR)
            }
            HandlerError::WrongTokenError => HttpResponse::new(http::StatusCode::UNAUTHORIZED),
            HandlerError::QueueFull => HttpResponse::new(StatusCode::TOO_MANY_REQUESTS),
        }
    }
}

async fn handle<A>(
    body: String,
    query: Query<TokenQuery>,
    state: Data<EndpointState<A>>,
) -> Result<String, HandlerError>
where
    A: Actor + Handler<EventMessage>,
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    let received_token = &query.token;
    trace!("Endpoint [{}] called with token [{}]", state.id, received_token);
    debug!("Received call with body [{}]", body);
    info!("Received {} bytes on webhook  {}", body.len(), state.id);

    let metric_attrs = &[KeyValue::new("id", state.id.clone())];
    state.metrics.webhooks_received.add(1, metric_attrs);
    state.metrics.bytes_received.add(body.len() as u64, metric_attrs);

    if !(state.token.eq(received_token)) {
        state.metrics.webhooks_failed.add(1, metric_attrs);
        error!("Endpoint [{}] - Token is not valid: [{}]", state.id, received_token);
        return Err(HandlerError::WrongTokenError);
    }

    let span = info_span!("processing_event", otel.name = "jmspath_collector");
    let process_result = {
        let _handle = span.enter();
        state.jmespath_collector.to_event(&body)
    };

    let event = match process_result {
        Ok(event) => event,
        Err(err) => {
            state.metrics.webhooks_failed.add(1, metric_attrs);
            error!("Endpoint {}: Error wile processing the payload: {}", &state.id, err);
            return Err(HandlerError::CollectorError { message: err.to_string() });
        }
    };

    let msg = EventMessage(TracedEvent { event, span: tracing::Span::current() });
    if state.actor_address.try_send(msg).is_err() {
        state.shared_metrics.events_dropped.add(1, &[]);
        error!("Endpoint {}: Dropping event because the queue is full.", &state.id);
        return Err(HandlerError::QueueFull);
    }

    Ok(state.id.to_string())
}
