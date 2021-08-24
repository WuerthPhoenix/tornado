use log::{debug, warn};
use tornado_common_logger::setup_logger;
use tornado_common_logger::LoggerConfig;
use tracing::{info, span, Level};
use tracing_futures::Instrument;

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
async fn should_keep_span_levels_with_spawn() -> Result<(), std::io::Error> {
    let config = LoggerConfig {
        stdout_output: true,
        level: "debug,logger_env_filter_setup_it::inner=info".to_owned(),
        file_output_path: None,
        tracing_elastic_apm: None,
    };

    let _guard = setup_logger(config).unwrap();

    let _span_1 = tracing::error_span!("level", "first").entered();

    debug!("main - this is debug");
    info!("main - this is info");
    warn!("main - this is warn");
    inner1::log_smt();

    let span_2 = tracing::error_span!("level", "second").entered();

    info!("main - this is level 2");

    inner2::log_smt(10, Data { id: 10 }).await;

    let handle = tokio::spawn(
        async move {
            inner2::log_smt(3, Data { id: 789 }).await;
        }
        .instrument(span_2.exit()),
    );

    handle.await.unwrap();

    Ok(())
}
