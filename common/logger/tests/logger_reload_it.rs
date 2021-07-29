use log::{debug, warn};
use tornado_common_logger::setup_logger;
use tornado_common_logger::LoggerConfig;
use tracing::info;

mod inner {
    use super::*;

    #[tracing::instrument(fields(yak))]
    pub async fn log_smt(yak: u32) {
        debug!("inner - yak: {} - this is debug", yak);
        info!("inner - yak: {} - this is info", yak);
        warn!("inner - yak: {} - this is warn", yak);
    }
}

#[tokio::test]
async fn should_setup_logger_with_env_filter() -> Result<(), std::io::Error> {
    let config = LoggerConfig {
        stdout_output: true,
        level: "debug,logger_reload_it::inner=warn".to_owned(),
        file_output_path: None,
        tracing_elastic_apm: None,
    };

    let guard = setup_logger(&config).unwrap();

    debug!("level debug - this is debug");
    info!("level debug - this is info");
    warn!("level debug - this is warn");
    inner::log_smt(11111).await;

    guard.reload("warn,logger_reload_it::inner=info").unwrap();

    debug!("level warn - this is debug");
    info!("level warn - this is info");
    warn!("level warn - this is warn");
    inner::log_smt(22222).await;

    Ok(())
}
