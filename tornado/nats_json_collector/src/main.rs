use actix::System;
use log::*;
use tornado_common_logger::elastic_apm::DEFAULT_APM_SERVER_CREDENTIALS_FILENAME;
use tornado_common_logger::setup_logger;
use tornado_nats_json_collector::*;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let arg_matches = config::arg_matches();

    let config_dir = arg_matches.value_of("config-dir").expect("config-dir should be provided");
    let topics_dir = arg_matches.value_of("topics-dir").expect("topics-dir should be provided");

    let mut collector_config = config::build_config(config_dir)?;
    if let Some(tracing_elastic_apm_config) = &mut collector_config.logger.tracing_elastic_apm {
        tracing_elastic_apm_config.read_apm_server_api_credentials_if_not_set(&format!(
            "{}/{}",
            config_dir, DEFAULT_APM_SERVER_CREDENTIALS_FILENAME
        ))?;
    }

    let _guard = setup_logger(&collector_config.logger)?;

    info!("Starting Nats JSON Collector");

    let full_topics_dir = format!("{}/{}", &config_dir, &topics_dir);
    let topics_config = config::read_topics_from_config(&full_topics_dir)?;

    start(collector_config.nats_json_collector, topics_config).await?;

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();

    Ok(())
}
