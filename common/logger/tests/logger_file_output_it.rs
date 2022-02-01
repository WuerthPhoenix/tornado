use log::{debug, warn};
use std::path::Path;
use std::{thread, time};
use tornado_common_logger::elastic_apm::ApmTracingConfig;
use tornado_common_logger::setup_logger;
use tornado_common_logger::LoggerConfig;
use tracing::info;

#[tokio::test]
async fn should_setup_logger_with_env_filter() -> Result<(), std::io::Error> {
    let tempdir = tempfile::tempdir().unwrap();
    let log_filename = format!(
        "{}/filename_{}.log",
        tempdir.path().to_str().unwrap().to_owned(),
        rand::random::<u64>()
    );
    let config = LoggerConfig {
        stdout_output: true,
        level: "debug,logger_file_output_it=info".to_owned(),
        file_output_path: Some(log_filename.clone()),
        tracing_elastic_apm: ApmTracingConfig::default(),
    };

    let _guard = setup_logger(config).unwrap();

    debug!("main - this is debug");
    info!("main - this is info");
    warn!("main - this is warn");

    let path = Path::new(&log_filename);
    assert!(path.exists());

    let mut found = false;
    let half_second_duration = time::Duration::from_millis(500);
    // The retry loop with a sleep of 0,5 sec is needed because in the CI this test could fail.
    // The problem is that the logs buffer could not be flushed yet when we search for them in the
    // log file.
    for _ in 1 .. 30 {
        let log_content = std::fs::read_to_string(path).unwrap();
        if log_content.contains("main - this is info") && !log_content.contains("main - this is debug") {
            found = true;
            break;
        }
        thread::sleep(half_second_duration);
    }
    assert!(found);

    Ok(())
}
