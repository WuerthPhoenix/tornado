use crate::Metrics;
use actix_web::web::Data;
use actix_web::{HttpResponse, Scope, web};
use std::sync::Arc;
use prometheus::{TextEncoder, Encoder};
use opentelemetry::metrics::MetricsError;

pub fn metrics_endpoints(metrics: Arc<Metrics>) -> Scope {
    web::scope("/metrics")
        .app_data::<Data<Metrics>>(metrics.into())
        .service(web::resource("/text").route(web::get().to(text_metrics)))
}

async fn text_metrics(metrics: Data<Metrics>) -> HttpResponse {
    let encoder = TextEncoder::new();
    let metric_families = metrics.prometheus_exporter.registry().gather();
    let mut buf = Vec::new();
    if let Err(err) = encoder.encode(&metric_families, &mut buf) {
        opentelemetry::global::handle_error(MetricsError::Other(err.to_string()));
    }

    //let body = String::from_utf8(buf).unwrap_or_default();
    HttpResponse::Ok()
        .insert_header((actix_web::http::header::CONTENT_TYPE, prometheus::TEXT_FORMAT))
        .body(buf)
}
