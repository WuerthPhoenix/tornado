use log::{debug, warn};
use tornado_common_logger::setup_logger;
use tornado_common_logger::LoggerConfig;
use tracing::{info, span, Level};

mod inner1 {
    use super::*;
    pub fn log_smt() {
        let yaks = 2;
        let span = span!(Level::WARN, "shaving_yaks", yaks);
        let _enter = span.enter();

        debug!("inner1 - this is debug");
        info!("inner1 - this is info");
        warn!("inner1 - this is warn. Yaks {}", yaks);
    }
}

mod inner2 {
    use super::*;

    #[tracing::instrument(skip(data), fields(id=data.id, show=true))]
    pub async fn log_smt(yak: u32, data: Data) {
        debug!("inner2 - id: {} - this is debug", data.id);
        info!("inner2 - id: {} - this is info", data.id);
        warn!("inner2 - id: {} - this is warn. Yak {}", data.id, yak);

        // info!(excitement = "yay!", "hello! I'm gonna shave a yak.");

        crate::inner1::log_smt();
    }
}

pub struct Data {
    id: u32,
}

#[tokio::test]
async fn should_setup_logger_with_env_filter() -> Result<(), std::io::Error> {
    let config = LoggerConfig {
        stdout_output: true,
        level: "debug,logger_env_filter_setup_it::inner=info".to_owned(),
        file_output_path: None,
        tracing_elastic_apm: Default::default(),
    };

    let _guard = setup_logger(&config, "./").unwrap();

    debug!("main - this is debug");
    info!("main - this is info");
    warn!("main - this is warn");
    inner1::log_smt();
    inner2::log_smt(3, Data { id: 789 }).await;

    Ok(())
}
