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

    HttpResponse::Ok()
        .insert_header((actix_web::http::header::CONTENT_TYPE, prometheus::TEXT_FORMAT))
        .body(buf)
}


#[cfg(test)]
mod test {

    use super::*;
    use actix_web::{test, App};
    use actix_web::http::StatusCode;
    use opentelemetry::Key;

    #[actix_rt::test]
    async fn should_expose_a_metrics_endpoint() {

        // Arrange
        let metrics = Arc::new(Metrics::new("tornado"));
        let mut srv = test::init_service(
            App::new().service(metrics_endpoints(metrics.clone())),
        )
            .await;

        // Record a metric
        {
            let meter = opentelemetry::global::meter("tornado");

            let http_requests_counter = meter
                .u64_counter("http_requests.counter")
                .init();

            let labels = vec![
                Key::from_static_str("test").string("something"),
            ];
            http_requests_counter.add(1, &labels);
        }

        let request =
            test::TestRequest::get().uri("/metrics/text").to_request();

        // Act
        let response = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(StatusCode::OK, response.status());

        let metrics = test::read_body(response).await;
        let content = std::str::from_utf8(&metrics).unwrap();
        assert!(content.contains(r#"test="something""#))
    }

}