use crate::config::DaemonCommandConfig;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::{web, HttpRequest, HttpResponse, Result, Scope};
use chrono::prelude::Local;
use serde::{Deserialize, Serialize};

pub fn monitoring_endpoints(scope: Scope, daemon_command_config: DaemonCommandConfig) -> Scope {
    scope
        .data(daemon_command_config)
        .service(web::resource("").route(web::get().to(index)))
        .service(web::resource("/ping").route(web::get().to(pong)))
        .service(
            web::resource("/communication_channel_config")
                .route(web::get().to(communication_channel_config)),
        )
}

async fn index(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().content_type("text/html").body(
        r##"
        <div>
            <h1>Available endpoints:</h1>
            <ul>
                <li><a href="/monitoring/ping">Ping</a></li>
                <li><a href="/monitoring/communication_channel_config">Communication Channel Config</a></li>
            </ul>
        </div>
        "##,
    )
}

#[derive(Serialize, Deserialize)]
pub struct PongResponse {
    pub message: String,
}

async fn pong(_req: HttpRequest) -> Result<Json<PongResponse>> {
    let dt = Local::now(); // e.g. `2014-11-28T21:45:59.324310806+09:00`
    let created_ms: String = dt.to_rfc3339();
    Ok(Json(PongResponse { message: format!("pong - {}", created_ms) }))
}

async fn communication_channel_config(
    daemon_command_config: Data<DaemonCommandConfig>,
) -> Result<Json<CommunicationChannelConfig>> {
    let event_tcp_socket_enabled = daemon_command_config.is_event_tcp_socket_enabled();
    let nats_enabled = daemon_command_config.is_nats_enabled();

    Ok(Json(CommunicationChannelConfig { event_tcp_socket_enabled, nats_enabled }))
}

#[derive(Serialize, Deserialize)]
pub struct CommunicationChannelConfig {
    pub event_tcp_socket_enabled: bool,
    pub nats_enabled: bool,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::AuthConfig;
    use actix_web::{test, App};
    use chrono::DateTime;

    #[actix_rt::test]
    async fn index_should_have_links_to_the_endpoints() {
        // Arrange
        let daemon_config = DaemonCommandConfig {
            event_tcp_socket_enabled: None,
            event_socket_ip: None,
            event_socket_port: None,
            nats_enabled: None,
            nats: None,
            web_server_ip: "".to_string(),
            web_server_port: 0,
            web_max_json_payload_size: None,
            message_queue_size: 0,
            thread_pool_config: None,
            retry_strategy: Default::default(),
            auth: AuthConfig::default(),
        };
        let mut srv = test::init_service(
            App::new().service(monitoring_endpoints(web::scope("/monitoring"), daemon_config)),
        )
        .await;

        // Act
        let request = test::TestRequest::get().uri("/monitoring").to_request();
        let response = test::read_response(&mut srv, request).await;

        // Assert
        let body = std::str::from_utf8(&response).unwrap();
        assert!(body.contains(r#"<a href="/monitoring/ping">"#));
        assert!(body.contains(
            r#"<a href="/monitoring/communication_channel_config">Communication Channel Config</a>"#
        ));
    }

    #[actix_rt::test]
    async fn ping_should_return_pong() {
        // Arrange
        let daemon_config = DaemonCommandConfig {
            event_tcp_socket_enabled: Some(true),
            event_socket_ip: None,
            event_socket_port: None,
            nats_enabled: Some(false),
            nats: None,
            web_server_ip: "".to_string(),
            web_server_port: 0,
            web_max_json_payload_size: None,
            message_queue_size: 0,
            thread_pool_config: None,
            retry_strategy: Default::default(),
            auth: AuthConfig::default(),
        };
        let mut srv = test::init_service(
            App::new().service(monitoring_endpoints(web::scope("/monitoring"), daemon_config)),
        )
        .await;

        // Act
        let request = test::TestRequest::get().uri("/monitoring/ping").to_request();

        // Assert
        let pong: PongResponse = test::read_response_json(&mut srv, request).await;
        assert!(pong.message.contains("pong - "));

        let date = DateTime::parse_from_rfc3339(&pong.message.clone()[7..]);
        // Assert
        assert!(date.is_ok());
    }

    #[actix_rt::test]
    async fn communication_ch_should_return_correct_configs() {
        // Arrange
        let daemon_config = DaemonCommandConfig {
            event_tcp_socket_enabled: Some(true),
            event_socket_ip: None,
            event_socket_port: None,
            nats_enabled: None,
            nats: None,
            web_server_ip: "".to_string(),
            web_server_port: 0,
            web_max_json_payload_size: None,
            message_queue_size: 0,
            thread_pool_config: None,
            retry_strategy: Default::default(),
            auth: AuthConfig::default(),
        };
        let mut srv = test::init_service(
            App::new().service(monitoring_endpoints(web::scope("/monitoring"), daemon_config)),
        )
        .await;

        // Act
        let request =
            test::TestRequest::get().uri("/monitoring/communication_channel_config").to_request();

        // Assert
        let channel_config: CommunicationChannelConfig =
            test::read_response_json(&mut srv, request).await;
        assert_eq!(channel_config.event_tcp_socket_enabled, true);
        assert_eq!(channel_config.nats_enabled, false);
    }
}
