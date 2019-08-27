use crate::config::WebhookConfig;
use actix::prelude::*;
use actix_web::{web, App, HttpRequest, HttpServer, Responder, Scope};
use chrono::prelude::Local;
use log::*;
use std::sync::Arc;
use tornado_collector_common::CollectorError;
use tornado_collector_jmespath::JMESPathEventCollector;
use tornado_common::actors::tcp_client::{EventMessage, TcpClientActor};
use tornado_common_api::Event;
use tornado_common_logger::setup_logger;

mod config;
mod handler;

fn pong(_req: HttpRequest) -> impl Responder {
    let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
    let created_ms: String = dt.to_rfc3339();
    format!("pong - {}", created_ms)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");
    let webhooks_dir =
        arg_matches.value_of("webhooks-dir").expect("webhooks-dir should be provided");

    let collector_config = config::build_config(&config_dir)?;

    setup_logger(&collector_config.logger).map_err(failure::Fail::compat)?;

    let webhooks_dir_full_path = format!("{}/{}", &config_dir, &webhooks_dir);
    let webhooks_config = config::read_webhooks_from_config(&webhooks_dir_full_path)
        .map_err(failure::Fail::compat)?;

    let port = collector_config.webhook_collector.server_port;
    let bind_address = collector_config.webhook_collector.server_bind_address.to_owned();

    System::run(move || {
        info!("Starting web server at port {}", port);

        // Start UdsWriter
        let tornado_tcp_address = format!(
            "{}:{}",
            collector_config.webhook_collector.tornado_event_socket_ip,
            collector_config.webhook_collector.tornado_event_socket_port
        );
        let tpc_client_addr = TcpClientActor::start_new(
            tornado_tcp_address.clone(),
            collector_config.webhook_collector.message_queue_size,
        );

        HttpServer::new(move || {
            App::new().service(
                create_app(webhooks_config.clone(), || {
                    let clone = tpc_client_addr.clone();
                    move |event| clone.do_send(EventMessage { event })
                })
                // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
                .unwrap_or_else(|err| {
                    error!("Cannot create the webhook handlers. Err: {}", err);
                    //System::current().stop_with_code(1);
                    std::process::exit(1);
                }),
            )
        })
        .bind(format!("{}:{}", bind_address, port))
        // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
        .unwrap_or_else(|err| {
            error!("Server cannot start on port {}. Err: {}", port, err);
            //System::current().stop_with_code(1);
            std::process::exit(1);
        })
        .start();
    })?;
    Ok(())
}

fn create_app<R: Fn(Event) + 'static, F: Fn() -> R>(
    webhooks_config: Vec<WebhookConfig>,
    factory: F,
) -> Result<Scope, CollectorError> {
    let mut scope = web::scope("");
    scope = scope.service(web::resource("/ping").route(web::get().to(pong)));

    for config in webhooks_config {
        let id = config.id.clone();
        let handler = Arc::new(handler::Handler {
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
        });

        let path = format!("/event/{}", config.id);
        info!("Creating endpoint: [{}]", &path);

        scope =
            scope.service(web::resource(&path).route(web::post().to(move |f| handler.handle(f))));
    }

    Ok(scope)
}

#[cfg(test)]
mod test {

    use super::*;
    use actix_web::{http, test};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;

    #[test]
    fn ping_should_return_pong() {
        // Arrange
        let mut srv =
            test::init_service(App::new().service(create_app(vec![], || |_| {}).unwrap()));

        // Act
        let request = test::TestRequest::get().uri("/ping").to_request();

        let response = test::read_response(&mut srv, request);

        // Assert
        let body = std::str::from_utf8(&response).unwrap();

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
        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), || |_| {}).unwrap()),
        );

        // Act
        let request_1 = test::TestRequest::post()
            .uri("/event/hook_1?token=hook_1_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response_1 = test::read_response(&mut srv, request_1);

        let request_2 = test::TestRequest::post()
            .uri("/event/hook_2?token=hook_2_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response_2 = test::read_response(&mut srv, request_2);

        // Assert
        let body_1 = std::str::from_utf8(&response_1).unwrap();
        assert_eq!("hook_1", body_1);

        let body_2 = std::str::from_utf8(&response_2).unwrap();
        assert_eq!("hook_2", body_2);
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
        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), || |_| {}).unwrap()),
        );

        // Act
        let request_1 = test::TestRequest::post()
            .uri("/event/hook_1?token=hook_1_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response_1 = test::call_service(&mut srv, request_1);

        let request_2 = test::TestRequest::post()
            .uri("/event/hook_2?token=WRONG_TOKEN")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response_2 = test::call_service(&mut srv, request_2);

        // Assert
        assert!(response_1.status().is_success());
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

        let mut srv = test::init_service(
            App::new().service(
                create_app(webhooks_config.clone(), || {
                    let clone = event.clone();
                    move |evt| {
                        let mut wrapper = clone.lock().unwrap();
                        *wrapper = Some(evt)
                    }
                })
                .unwrap(),
            ),
        );

        // Act
        let request_1 = test::TestRequest::post()
            .uri("/event/hook_1?token=hook_1_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload(
                r#"{
                    "map" : {
                        "first": "webhook_event"
                    }
                }"#,
            )
            .to_request();
        let response_1 = test::read_response(&mut srv, request_1);

        // Assert
        let body_1 = std::str::from_utf8(&response_1).unwrap();
        assert_eq!("hook_1", body_1);

        let value = event_clone.lock().unwrap();
        assert_eq!("webhook_event", value.as_ref().unwrap().event_type)
    }

    #[test]
    fn should_return_404_if_hook_does_not_exists() {
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

        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), || |_| {}).unwrap()),
        );

        // Act
        let request = test::TestRequest::post()
            .uri("/event/hook_2?token=hook_2_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response = test::call_service(&mut srv, request);;

        // Assert
        assert_eq!(http::StatusCode::NOT_FOUND, response.status());
    }

    #[test]
    fn should_return_405_if_get_instead_of_post() {
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

        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), || |_| {}).unwrap()),
        );

        // Act
        let request = test::TestRequest::get()
            .uri("/event/hook_1?token=hook_1_token")
            .header(http::header::CONTENT_TYPE, "application/json")
            .to_request();
        let response = test::call_service(&mut srv, request);;

        // Assert
        assert_eq!(http::StatusCode::METHOD_NOT_ALLOWED, response.status());
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

        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), || |_| {}).unwrap()),
        );

        // Act
        let request = test::TestRequest::post()
            .uri("/event/hook%20with%20space?token=token%26%23%3F%3D")
            .header(http::header::CONTENT_TYPE, "application/json")
            .set_payload("{}")
            .to_request();
        let response = test::call_service(&mut srv, request);;

        // Assert
        assert_eq!(http::StatusCode::OK, response.status());
    }

}
