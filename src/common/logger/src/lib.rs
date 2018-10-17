extern crate chrono;
extern crate fern;
extern crate log;

pub mod config;

use std::str::FromStr;

pub fn setup_logger(logger_config: &config::LoggerConfig) -> Result<(), fern::InitError> {
    let mut log_dispatcher = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        }).level(log::LevelFilter::from_str(&logger_config.root_level).unwrap());

    // ToDo: if needed, add per module logger config here
    // .level_for("tornado", log::LevelFilter::from_str(&logger_config.level).unwrap());

    if logger_config.output_system_enabled {
        log_dispatcher = log_dispatcher.chain(std::io::stdout());
    }

    if logger_config.output_file_enabled {
        log_dispatcher = log_dispatcher.chain(fern::log_file(&logger_config.output_file_name)?);
    }

    log_dispatcher.apply()?;

    Ok(())
}
