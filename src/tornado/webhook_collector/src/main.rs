use crate::config::WebhookConfig;
use actix::prelude::*;
use actix_web::http::Method;
use actix_web::{server, App, HttpRequest, Responder};
use chrono::prelude::Local;
use log::*;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common_logger::setup_logger;

mod actors;
mod config;
mod handler;

fn pong(_req: &HttpRequest) -> impl Responder {
    let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
    let created_ts: String = dt.to_rfc3339();
    format!("pong - {}", created_ts)
}

fn main() {
    let config = config::Conf::build();

    setup_logger(&config.logger).expect("Cannot configure the logger");

    let webhooks_dir = format!("{}/{}", &config.io.config_dir, &config.io.webhooks_dir);
    let webhooks_config = config::read_webhooks_from_config(&webhooks_dir)
        .expect("Cannot parse the webhooks configuration");

    let port = config.io.server_port;

    System::run(move || {
        info!("Starting web server at port {}", port);

        // Start UdsWriter
        let uds_writer_addr = actors::uds_writer::UdsWriterActor::start_new(
            config.io.uds_path.clone(),
            config.io.uds_mailbox_capacity,
        );

        server::new(move || create_app(webhooks_config.clone()))
            .bind(format!("0.0.0.0:{}", port))
            .unwrap_or_else(|err| panic!("Server cannot start on port {}. Err: {}", port, err))
            .start();

    });
}

fn create_app(webhooks_config: Vec<WebhookConfig>) -> App {
    let mut app = App::new().resource("/ping", |r| r.method(Method::GET).f(pong));

    for config in webhooks_config {
        let id = config.id.clone();
        let handler = handler::Handler {
            id: config.id.clone(),
            token: config.token,
            collector: JMESPathEventCollector::build(config.collector_config).unwrap_or_else(
                |err| panic!("Cannot create collector for webhook with id [{}]. Err: {}", id, err),
            ),
        };
        let path = format!("/event/{}", config.id);
        info!("Creating endpoint: [{}]", &path);
        app = app.resource(&path, |r| r.method(Method::POST).with(move |f| handler.handle(f)));
    }

    app
}

#[cfg(test)]
mod test {

    use super::*;
    use actix_web::test::TestServer;
    use actix_web::{http, HttpMessage};
    use std::collections::HashMap;
    use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;

    #[test]
    fn ping_should_return_pong() {
        // Arrange
        let mut srv = TestServer::with_factory(|| create_app(vec![]));

        // Act
        let request = srv.client(http::Method::GET, "/ping").finish().unwrap();
        let response = srv.execute(request.send()).unwrap();

        // Assert
        assert!(response.status().is_success());

        let bytes = srv.execute(response.body()).unwrap();
        let body = std::str::from_utf8(&bytes).unwrap();

        assert!(body.contains("pong - "));
    }

    #[test]
    fn should_create_a_path_per_webhook() {
        // Arrange
        let mut webhooks_config = vec![];
        webhooks_config.push(WebhookConfig {
            id: "hook_1".to_owned(),
            token: "hook_1_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "hook_1_type".to_owned(),
                payload: HashMap::new(),
            },
        });
        webhooks_config.push(WebhookConfig {
            id: "hook_2".to_owned(),
            token: "hook_2_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "hook_2_type".to_owned(),
                payload: HashMap::new(),
            },
        });
        let mut srv = TestServer::with_factory(move || create_app(webhooks_config.clone()));

        // Act
        let request_1 = srv
            .client(http::Method::POST, "/event/hook_1?token=hook_1_token")
            .content_type("application/json")
            .body("{}")
            .unwrap();
        let response_1 = srv.execute(request_1.send()).unwrap();

        let request_2 = srv
            .client(http::Method::POST, "/event/hook_2?token=hook_2_token")
            .content_type("application/json")
            .body("{}")
            .unwrap();
        let response_2 = srv.execute(request_2.send()).unwrap();

        // Assert
        assert!(response_1.status().is_success());
        let body_1 =
            std::str::from_utf8(&srv.execute(response_1.body()).unwrap()).unwrap().to_owned();
        assert_eq!("hook_1", &body_1);

        assert!(response_2.status().is_success());
        let body_2 =
            std::str::from_utf8(&srv.execute(response_2.body()).unwrap()).unwrap().to_owned();
        assert_eq!("hook_2", &body_2);
    }

    #[test]
    fn should_accept_calls_only_if_token_matches() {
        // Arrange
        let mut webhooks_config = vec![];
        webhooks_config.push(WebhookConfig {
            id: "hook_1".to_owned(),
            token: "hook_1_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "hook_1_type".to_owned(),
                payload: HashMap::new(),
            },
        });
        webhooks_config.push(WebhookConfig {
            id: "hook_2".to_owned(),
            token: "hook_2_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "hook_2_type".to_owned(),
                payload: HashMap::new(),
            },
        });
        let mut srv = TestServer::with_factory(move || create_app(webhooks_config.clone()));

        // Act
        let request_1 = srv
            .client(http::Method::POST, "/event/hook_1?token=hook_1_token")
            .content_type("application/json")
            .body("{}")
            .unwrap();
        let response_1 = srv.execute(request_1.send()).unwrap();

        let request_2 = srv
            .client(http::Method::POST, "/event/hook_2?token=WRONG_TOKEN")
            .content_type("application/json")
            .body("{}")
            .unwrap();
        let response_2 = srv.execute(request_2.send()).unwrap();

        // Assert
        assert!(response_1.status().is_success());
        let body_1 =
            std::str::from_utf8(&srv.execute(response_1.body()).unwrap()).unwrap().to_owned();
        assert_eq!("hook_1", &body_1);

        assert_eq!(http::StatusCode::UNAUTHORIZED, response_2.status());
    }
}
