use crate::actors::uds_writer::EventMessage;
use crate::config::WebhookConfig;
use actix::prelude::*;
use actix_web::http::Method;
use actix_web::{server, App, HttpRequest, Responder};
use chrono::prelude::Local;
use failure::Fail;
use log::*;
use tornado_collector_common::CollectorError;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common::actors;
use tornado_common_api::Event;
use tornado_common_logger::setup_logger;
use url::percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};

mod config;
mod handler;

fn pong(_req: &HttpRequest) -> impl Responder {
    let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
    let created_ts: String = dt.to_rfc3339();
    format!("pong - {}", created_ts)
}

fn main() -> Result<(), Box<std::error::Error>> {
    let config = config::Conf::build();

    setup_logger(&config.logger).map_err(|err| err.compat())?;

    let webhooks_dir = format!("{}/{}", &config.io.config_dir, &config.io.webhooks_dir);
    let webhooks_config =
        config::read_webhooks_from_config(&webhooks_dir).map_err(|err| err.compat())?;

    let port = config.io.server_port;
    let bind_address = config.io.bind_address.to_owned();

    System::run(move || {
        info!("Starting web server at port {}", port);

        // Start UdsWriter
        let uds_writer_addr = actors::uds_writer::UdsWriterActor::start_new(
            config.io.uds_path.clone(),
            config.io.uds_mailbox_capacity,
        );

        server::new(move || {
            create_app(webhooks_config.clone(), || {
                let clone = uds_writer_addr.clone();
                move |event| clone.do_send(EventMessage { event })
            })
            // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
            .unwrap_or_else(|err| {
                error!("Cannot create the webhook handlers. Err: {}", err);
                //System::current().stop_with_code(1);
                std::process::exit(1);
            })
        })
        .bind(format!("{}:{}", bind_address, port))
        // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
        .unwrap_or_else(|err| {
            error!("Server cannot start on port {}. Err: {}", port, err);
            //System::current().stop_with_code(1);
            std::process::exit(1);
        })
        .start();
    });
    Ok(())
}

fn create_app<R: Fn(Event) + 'static, F: Fn() -> R>(
    webhooks_config: Vec<WebhookConfig>,
    factory: F,
) -> Result<App, CollectorError> {
    let mut app = App::new().resource("/ping", |r| r.method(Method::GET).f(pong));

    for config in webhooks_config {
        let id = config.id.clone();
        let handler = handler::Handler {
            id: config.id.clone(),
            token: config.token,
            collector: JMESPathEventCollector::build(config.collector_config).map_err(|err| {
                CollectorError::CollectorCreationError {
                    message: format!(
                        "Cannot create collector for webhook with id [{}]. Err: {}",
                        id, err
                    ),
                }
            })?,
            callback: factory(),
        };

        let url_encoded_id: String = utf8_percent_encode(&config.id, DEFAULT_ENCODE_SET).collect();
        let path = format!("/event/{}", url_encoded_id);
        info!("Creating endpoint: [{}]", &path);
        app = app.resource(&path, move |r| r.method(Method::POST).with(move |f| handler.handle(f)));
    }

    Ok(app)
}

#[cfg(test)]
mod test {

    use super::*;
    use actix_web::test::TestServer;
    use actix_web::{http, HttpMessage};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;

    #[test]
    fn ping_should_return_pong() {
        // Arrange
        let mut srv = TestServer::with_factory(|| create_app(vec![], || |_| {}).unwrap());

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
        let mut srv = TestServer::with_factory(move || {
            create_app(webhooks_config.clone(), || |_| {}).unwrap()
        });

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
        let mut srv = TestServer::with_factory(move || {
            create_app(webhooks_config.clone(), || |_| {}).unwrap()
        });

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

    #[test]
    fn should_call_the_callback_on_each_event() {
        // Arrange
        let mut webhooks_config = vec![];

        webhooks_config.push(WebhookConfig {
            id: "hook_1".to_owned(),
            token: "hook_1_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "${map.first}".to_owned(),
                payload: HashMap::new(),
            },
        });

        let event = Arc::new(Mutex::new(None));
        let event_clone = event.clone();
        let mut srv = TestServer::with_factory(move || {
            create_app(webhooks_config.clone(), || {
                let clone = event.clone();
                move |evt| {
                    let mut wrapper = clone.lock().unwrap();
                    *wrapper = Some(evt)
                }
            })
            .unwrap()
        });

        // Act
        let request_1 = srv
            .client(http::Method::POST, "/event/hook_1?token=hook_1_token")
            .content_type("application/json")
            .body(
                r#"{
                    "map" : {
                        "first": "webhook_event"
                    }
                }"#,
            )
            .unwrap();
        let response_1 = srv.execute(request_1.send()).unwrap();

        // Assert
        assert!(response_1.status().is_success());
        let body_1 =
            std::str::from_utf8(&srv.execute(response_1.body()).unwrap()).unwrap().to_owned();
        assert_eq!("hook_1", &body_1);

        let value = event_clone.lock().unwrap();
        assert_eq!("webhook_event", value.as_ref().unwrap().event_type)
    }

    #[test]
    fn should_return_404_if_get_instead_of_post() {
        // Arrange
        let mut webhooks_config = vec![];

        webhooks_config.push(WebhookConfig {
            id: "hook_1".to_owned(),
            token: "hook_1_token".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "${map.first}".to_owned(),
                payload: HashMap::new(),
            },
        });

        let mut srv = TestServer::with_factory(move || {
            create_app(webhooks_config.clone(), || |_| {}).unwrap()
        });

        // Act
        let request = srv
            .client(http::Method::GET, "/event/hook_1?token=hook_1_token")
            .content_type("application/json")
            .finish()
            .unwrap();
        let response = srv.execute(request.send()).unwrap();

        // Assert
        assert_eq!(http::StatusCode::NOT_FOUND, response.status());
    }

    #[test]
    fn should_url_encode_id_and_token() {
        // Arrange
        let mut webhooks_config = vec![];

        webhooks_config.push(WebhookConfig {
            id: "hook with space".to_owned(),
            token: "token&#?=".to_owned(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "type".to_owned(),
                payload: HashMap::new(),
            },
        });

        let mut srv = TestServer::with_factory(move || {
            create_app(webhooks_config.clone(), || |_| {}).unwrap()
        });

        // Act
        let request_1 = srv
            .client(http::Method::POST, "/event/hook%20with%20space?token=token%26%23%3F%3D")
            .content_type("application/json")
            .body("{}")
            .unwrap();
        let response_1 = srv.execute(request_1.send()).unwrap();

        // Assert
        assert!(response_1.status().is_success());
        let body_1 =
            std::str::from_utf8(&srv.execute(response_1.body()).unwrap()).unwrap().to_owned();
        assert_eq!("hook with space", &body_1);
    }
}
