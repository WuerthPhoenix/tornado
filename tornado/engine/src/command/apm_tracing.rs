use crate::config::{EnableOrDisableSubCommand, build_config};
use ajars::reqwest::reqwest::ClientBuilder;
use ajars::reqwest::AjarsReqwest;
use tornado_engine_api::runtime_config::web::RUNTIME_CONFIG_ENDPOINT_V1_BASE;
use tornado_engine_api_dto::runtime_config::{SET_APM_PRIORITY_CONFIG_REST, SetApmPriorityConfigurationRequestDto, SET_STDOUT_PRIORITY_CONFIG_REST, SetStdoutPriorityConfigurationRequestDto};

pub async fn apm_tracing(
    config_dir: &str,
    command: &EnableOrDisableSubCommand
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {

    let global_config = build_config(config_dir)?;
    let daemon_config = global_config.tornado.daemon;

    call_apm_tracing_endpoint(&daemon_config.web_server_ip, daemon_config.web_server_port, command).await

}


async fn call_apm_tracing_endpoint(
    web_server_ip: &str,
    web_server_port: u16,
    command: &EnableOrDisableSubCommand
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Set apm-tracing to: {:?}", command);

    let web_server_ip = {
        if web_server_ip.eq("0.0.0.0") {
            "127.0.0.1"
        } else {
            web_server_ip
        }
    };

    let base_url = format!("http://{}:{}/api/{}", web_server_ip, web_server_port, RUNTIME_CONFIG_ENDPOINT_V1_BASE);
    println!("Using tornado base endpoint: {}", base_url);
    let ajars = AjarsReqwest::new(ClientBuilder::new().build()?, base_url);

    match command {
        EnableOrDisableSubCommand::Enable => {
            ajars
                .request(&SET_APM_PRIORITY_CONFIG_REST)
                .send(&SetApmPriorityConfigurationRequestDto {
                    logger_level: None
                })
                .await?;
        },
        EnableOrDisableSubCommand::Disable => {
            ajars
                .request(&SET_STDOUT_PRIORITY_CONFIG_REST)
                .send(&SetStdoutPriorityConfigurationRequestDto {})
                .await?;
        }
    }

    println!("Apm-tracing correctly set.");
    Ok(())
}