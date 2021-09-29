use crate::config::{build_config, EnableOrDisableSubCommand};
use ajars::reqwest::reqwest::ClientBuilder;
use ajars::reqwest::AjarsReqwest;
use tornado_engine_api::runtime_config::web::RUNTIME_CONFIG_ENDPOINT_V1_BASE;
use tornado_engine_api_dto::auth::Auth;
use tornado_engine_api_dto::runtime_config::{
    SetApmPriorityConfigurationRequestDto, SetStdoutPriorityConfigurationRequestDto,
    SET_APM_PRIORITY_CONFIG_REST, SET_STDOUT_PRIORITY_CONFIG_REST,
};

pub async fn apm_tracing(
    config_dir: &str,
    command: &EnableOrDisableSubCommand,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let global_config = build_config(config_dir)?;
    let daemon_config = global_config.tornado.daemon;

    call_apm_tracing_endpoint(&daemon_config.web_server_ip, daemon_config.web_server_port, command)
        .await
}

async fn call_apm_tracing_endpoint(
    web_server_ip: &str,
    web_server_port: u16,
    command: &EnableOrDisableSubCommand,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Set apm-tracing to: {:?}", command);

    let web_server_ip = {
        if web_server_ip.eq("0.0.0.0") {
            "127.0.0.1"
        } else {
            web_server_ip
        }
    };

    let base_url = format!(
        "http://{}:{}/api{}",
        web_server_ip, web_server_port, RUNTIME_CONFIG_ENDPOINT_V1_BASE
    );
    println!("Using tornado base endpoint: {}", base_url);

    let ajars = AjarsReqwest::new(ClientBuilder::new().build()?, base_url);

    let auth_header = base64::encode(serde_json::to_string(&Auth {
        user: "tornado".to_owned(),
        roles: vec!["admin".to_owned()],
        preferences: None,
    })?);

    match command {
        EnableOrDisableSubCommand::Enable => {
            ajars
                .request(&SET_APM_PRIORITY_CONFIG_REST)
                .bearer_auth(auth_header)
                .send(&SetApmPriorityConfigurationRequestDto { logger_level: None })
                .await?;
        }
        EnableOrDisableSubCommand::Disable => {
            ajars
                .request(&SET_STDOUT_PRIORITY_CONFIG_REST)
                .bearer_auth(auth_header)
                .send(&SetStdoutPriorityConfigurationRequestDto {})
                .await?;
        }
    }

    println!("Apm-tracing correctly set.");
    Ok(())
}

#[cfg(test)]
mod test {

    use super::*;
    use ajars::RestType;
    use httpmock::Method::POST;
    use httpmock::MockServer;

    #[tokio::test]
    async fn should_call_the_apm_priority_endpoint() {
        // Arrange
        let server = MockServer::start();
        let endpoint = server.mock(|when, then| {
            when.method(POST).path(format!(
                "/api{}{}",
                RUNTIME_CONFIG_ENDPOINT_V1_BASE,
                SET_APM_PRIORITY_CONFIG_REST.path()
            ));
            then.json_body(()).status(200);
        });

        let port = server.port();

        // Act
        call_apm_tracing_endpoint("127.0.0.1", port, &EnableOrDisableSubCommand::Enable)
            .await
            .unwrap();

        // Assert
        endpoint.assert_hits(1);
    }

    #[tokio::test]
    async fn should_call_the_stdout_priority_endpoint() {
        // Arrange
        let server = MockServer::start();
        let endpoint = server.mock(|when, then| {
            when.method(POST).path(format!(
                "/api{}{}",
                RUNTIME_CONFIG_ENDPOINT_V1_BASE,
                SET_STDOUT_PRIORITY_CONFIG_REST.path()
            ));
            then.json_body(()).status(200);
        });

        let port = server.port();

        // Act
        call_apm_tracing_endpoint("127.0.0.1", port, &EnableOrDisableSubCommand::Disable)
            .await
            .unwrap();

        // Assert
        endpoint.assert_hits(1);
    }
}
