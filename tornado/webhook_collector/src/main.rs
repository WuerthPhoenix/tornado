use std::num::NonZeroU16;
use std::sync::Arc;

use crate::config::WebhookConfig;
use crate::handler::{create_app, create_endpoint_state};
use actix::dev::ToEnvelope;
use actix::{Actor, Addr};
use actix_web::http::KeepAlive;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use log::*;
use tornado_common::actors::message::EventMessage;
use tornado_common::actors::nats_publisher::NatsPublisherActor;
use tornado_common::actors::tcp_client::TcpClientActor;
use tornado_common::actors::TornadoConnectionChannel;
use tornado_common::TornadoError;
use tornado_common_logger::elastic_apm::DEFAULT_APM_SERVER_CREDENTIALS_FILENAME;
use tornado_common_logger::setup_logger;
use tornado_common_metrics::Metrics;
use tracing_actix_web::TracingLogger;

mod config;
mod handler;

const APP_NAME: &str = "tornado_webhook_collector";

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");
    let webhooks_dir =
        arg_matches.value_of("webhooks-dir").expect("webhooks-dir should be provided");

    let mut collector_config = config::build_config(config_dir)?;
    let apm_server_api_credentials_filepath =
        format!("{}/{}", config_dir, DEFAULT_APM_SERVER_CREDENTIALS_FILENAME);
    // Get the result and log the error later because the logger is not available yet
    let apm_credentials_read_result = collector_config
        .logger
        .tracing_elastic_apm
        .read_apm_server_api_credentials_if_not_set(&apm_server_api_credentials_filepath);

    let _guard = setup_logger(collector_config.logger)?;
    if let Err(apm_credentials_read_error) = apm_credentials_read_result {
        warn!("{:?}", apm_credentials_read_error);
    }

    let webhooks_dir_full_path = format!("{}/{}", &config_dir, &webhooks_dir);
    let webhooks_config = config::read_webhooks_from_config(&webhooks_dir_full_path)?;

    let workers = collector_config.webhook_collector.workers;

    let port = collector_config.webhook_collector.server_port;
    let bind_address = collector_config.webhook_collector.server_bind_address.to_owned();

    info!("Starting web server at port {}", port);

    //
    // WARN:
    // This 'if' block contains some duplicated code to allow temporary compatibility with the config file format of the previous release.
    // It will be removed in the next release when the `tornado_connection_channel` will be mandatory.
    //
    if let (Some(tornado_event_socket_ip), Some(tornado_event_socket_port)) = (
        collector_config.webhook_collector.tornado_event_socket_ip,
        collector_config.webhook_collector.tornado_event_socket_port,
    ) {
        info!("Connect to Tornado through TCP socket");
        // Start TcpWriter
        let tornado_tcp_address =
            format!("{}:{}", tornado_event_socket_ip, tornado_event_socket_port,);

        let actor_address = TcpClientActor::start_new(
            tornado_tcp_address,
            collector_config.webhook_collector.message_queue_size,
        );
        start_http_server(actor_address, webhooks_config, bind_address, port, workers).await?;
    } else if let Some(connection_channel) =
        collector_config.webhook_collector.tornado_connection_channel
    {
        match connection_channel {
            TornadoConnectionChannel::Nats { nats } => {
                info!("Connect to Tornado through NATS");
                let actor_address = NatsPublisherActor::start_new(
                    nats,
                    collector_config.webhook_collector.message_queue_size,
                )
                .await?;
                start_http_server(actor_address, webhooks_config, bind_address, port, workers)
                    .await?;
            }
            TornadoConnectionChannel::Tcp { tcp_socket_ip, tcp_socket_port } => {
                info!("Connect to Tornado through TCP socket");
                // Start TcpWriter
                let tornado_tcp_address = format!("{}:{}", tcp_socket_ip, tcp_socket_port,);

                let actor_address = TcpClientActor::start_new(
                    tornado_tcp_address,
                    collector_config.webhook_collector.message_queue_size,
                );
                start_http_server(actor_address, webhooks_config, bind_address, port, workers)
                    .await?;
            }
        };
    } else {
        return Err(TornadoError::ConfigurationError {
            message: "A communication channel must be specified.".to_owned(),
        }
        .into());
    }

    Ok(())
}

async fn start_http_server<A: Actor + actix::Handler<EventMessage>>(
    actor_address: Addr<A>,
    webhooks_config: Vec<WebhookConfig>,
    bind_address: String,
    port: u32,
    workers: Option<NonZeroU16>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    <A as Actor>::Context: ToEnvelope<A, EventMessage>,
{
    let metrics = Arc::new(Metrics::new(APP_NAME));
    let endpoints = create_endpoint_state(webhooks_config, actor_address)?;

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(TracingLogger::default())
            .service(create_app(endpoints.clone(), metrics.clone()))
    })
    .keep_alive(KeepAlive::Disabled)
    .bind(format!("{}:{}", bind_address, port))
    // here we are forced to unwrap by the Actix API. See: https://github.com/actix/actix/issues/203
    .unwrap_or_else(|err| {
        error!("Server cannot start on port {}. Err: {:?}", port, err);
        std::process::exit(1);
    });

    if let Some(workers) = workers {
        info!("Setting up {} workers", workers);
        srv = srv.workers(workers.get() as usize);
    }

    srv.run().await?;

    Ok(())
}

#[cfg(test)]
mod test {

    use crate::config::default_webhook_config_max_payload_size;

    use super::*;
    use actix_web::{http, test};
    use human_units::Size;
    use std::collections::HashMap;
    use tornado_collector_jmespath::config::JMESPathEventCollectorConfig;

    fn test_webhook_config() -> WebhookConfig {
        WebhookConfig {
            id: "hook_1".to_owned(),
            token: "hook_1_token".to_owned(),
            max_payload_size: default_webhook_config_max_payload_size(),
            collector_config: JMESPathEventCollectorConfig {
                event_type: "hook_1_type".to_owned(),
                payload: HashMap::new(),
            },
        }
    }

    #[actix_rt::test]
    async fn ping_should_return_pong() {
        // Arrange
        let addr = actix::actors::mocker::Mocker::<EventMessage>::mock(Box::new(|b, _| b)).start();
        let srv = test::init_service(App::new().service(create_app(vec![], addr).unwrap())).await;

        // Act
        let request = test::TestRequest::get().uri("/ping").to_request();

        let response = test::call_and_read_body(&srv, request).await;

        // Assert
        let body = std::str::from_utf8(&response).unwrap();

        assert!(body.contains("pong - "));
    }

    #[actix_rt::test]
    async fn should_create_a_path_per_webhook() {
        // Arrange
        let webhooks_config = vec![
            WebhookConfig { ..test_webhook_config() },
            WebhookConfig {
                id: "hook_2".to_owned(),
                token: "hook_2_token".to_owned(),
                collector_config: JMESPathEventCollectorConfig {
                    event_type: "hook_2_type".to_owned(),
                    payload: HashMap::new(),
                },
                ..test_webhook_config()
            },
        ];

        let addr = actix::actors::mocker::Mocker::<EventMessage>::mock(Box::new(|b, _| b)).start();
        let srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), addr).unwrap()),
        )
        .await;

        // Act
        let request_1 = test::TestRequest::post()
            .uri("/event/hook_1?token=hook_1_token")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .set_payload("{}")
            .to_request();
        let response_1 = test::call_and_read_body(&srv, request_1).await;

        let request_2 = test::TestRequest::post()
            .uri("/event/hook_2?token=hook_2_token")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .set_payload("{}")
            .to_request();
        let response_2 = test::call_and_read_body(&srv, request_2).await;

        // Assert
        let body_1 = std::str::from_utf8(&response_1).unwrap();
        assert_eq!("hook_1", body_1);

        let body_2 = std::str::from_utf8(&response_2).unwrap();
        assert_eq!("hook_2", body_2);
    }

    #[actix_rt::test]
    async fn should_accept_calls_only_if_token_matches() {
        // Arrange
        let webhooks_config = vec![
            WebhookConfig { ..test_webhook_config() },
            WebhookConfig {
                id: "hook_2".to_owned(),
                token: "hook_2_token".to_owned(),
                collector_config: JMESPathEventCollectorConfig {
                    event_type: "hook_2_type".to_owned(),
                    payload: HashMap::new(),
                },
                ..test_webhook_config()
            },
        ];

        let addr = actix::actors::mocker::Mocker::<EventMessage>::mock(Box::new(|b, _| b)).start();
        let srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), addr).unwrap()),
        )
        .await;

        // Act
        let request_1 = test::TestRequest::post()
            .uri("/event/hook_1?token=hook_1_token")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .set_payload("{}")
            .to_request();
        let response_1 = test::call_service(&srv, request_1).await;

        let request_2 = test::TestRequest::post()
            .uri("/event/hook_2?token=WRONG_TOKEN")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .set_payload("{}")
            .to_request();
        let response_2 = test::call_service(&srv, request_2).await;

        // Assert
        assert!(response_1.status().is_success());
        assert_eq!(http::StatusCode::UNAUTHORIZED, response_2.status());
    }

    #[actix_rt::test]
    async fn should_return_404_if_hook_does_not_exists() {
        // Arrange
        let webhooks_config =
            vec![WebhookConfig { id: "hook_1".to_owned(), ..test_webhook_config() }];

        let addr = actix::actors::mocker::Mocker::<EventMessage>::mock(Box::new(|b, _| b)).start();
        let srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), addr).unwrap()),
        )
        .await;

        // Act
        let request = test::TestRequest::post()
            .uri("/event/hook_2?token=hook_2_token")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .set_payload("{}")
            .to_request();
        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(http::StatusCode::NOT_FOUND, response.status());
    }

    #[actix_rt::test]
    async fn should_return_405_if_get_instead_of_post() {
        // Arrange
        let webhooks_config =
            vec![WebhookConfig { id: "hook_1".to_owned(), ..test_webhook_config() }];

        let addr = actix::actors::mocker::Mocker::<EventMessage>::mock(Box::new(|b, _| b)).start();
        let srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), addr).unwrap()),
        )
        .await;

        // Act
        let request = test::TestRequest::get()
            .uri("/event/hook_1?token=hook_1_token")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .to_request();
        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(http::StatusCode::METHOD_NOT_ALLOWED, response.status());
    }

    #[actix_rt::test]
    async fn should_url_encode_id_and_token() {
        // Arrange
        let webhooks_config = vec![WebhookConfig {
            id: "hook with space".to_owned(),
            token: "token&#?=".to_owned(),
            ..test_webhook_config()
        }];

        let addr = actix::actors::mocker::Mocker::<EventMessage>::mock(Box::new(|b, _| b)).start();
        let srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), addr).unwrap()),
        )
        .await;

        // Act
        let request = test::TestRequest::post()
            .uri("/event/hook%20with%20space?token=token%26%23%3F%3D")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .set_payload("{}")
            .to_request();
        let response = test::call_service(&srv, request).await;

        // Assert
        assert_eq!(http::StatusCode::OK, response.status());
    }

    #[actix_rt::test]
    async fn should_refuse_large_payload() {
        // Arrange
        let mut webhooks_config = vec![];

        webhooks_config.push(WebhookConfig {
            id: "limit_payload".to_owned(),
            token: "123".to_owned(),
            max_payload_size: Size(1024 * 512), // 512 KB
            ..test_webhook_config()
        });

        webhooks_config.push(WebhookConfig {
            id: "default_payload".to_owned(),
            token: "123".to_owned(),
            ..test_webhook_config()
        });

        let addr = actix::actors::mocker::Mocker::<EventMessage>::mock(Box::new(|b, _| b)).start();
        let mut srv = test::init_service(
            App::new().service(create_app(webhooks_config.clone(), addr).unwrap()),
        )
        .await;

        let json = vec![b'{', b'}'];
        let payload_1kb = [vec![b' '; 1024], json.clone()].concat();
        let payload_1mb = [vec![b' '; 1024 * 1024], json.clone()].concat();
        let payload_10mb = [vec![b' '; 1024 * 1024 * 10], json.clone()].concat();

        // Act
        let request = test::TestRequest::post()
            .uri("/event/limit_payload?token=123")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .set_payload(payload_1kb)
            .to_request();
        let response_in_limit = test::call_service(&mut srv, request).await;

        let request = test::TestRequest::post()
            .uri("/event/limit_payload?token=123")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .set_payload(payload_1mb.clone())
            .to_request();
        let response_over_limit = test::call_service(&mut srv, request).await;

        let request = test::TestRequest::post()
            .uri("/event/default_payload?token=123")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .set_payload(payload_1mb)
            .to_request();
        let response_in_default_limit = test::call_service(&mut srv, request).await;

        let request = test::TestRequest::post()
            .uri("/event/default_payload?token=123")
            .insert_header((http::header::CONTENT_TYPE, "application/json"))
            .set_payload(payload_10mb)
            .to_request();
        let response_over_default_limit = test::call_service(&mut srv, request).await;

        // Assert
        assert_eq!(http::StatusCode::OK, response_in_limit.status());
        assert_eq!(http::StatusCode::PAYLOAD_TOO_LARGE, response_over_limit.status());
        assert_eq!(http::StatusCode::OK, response_in_default_limit.status());
        assert_eq!(http::StatusCode::PAYLOAD_TOO_LARGE, response_over_default_limit.status());
    }
}
