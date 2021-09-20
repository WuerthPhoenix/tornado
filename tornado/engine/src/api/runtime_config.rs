use async_trait::async_trait;
use tornado_engine_api::error::ApiError;
use tornado_engine_api::runtime_config::api::RuntimeConfigApiHandler;
use std::sync::Arc;
use tornado_common_logger::LogWorkerGuard;
use tornado_engine_api_dto::runtime_config::{LoggerConfigDto, SetLoggerApmRequestDto, SetLoggerLevelRequestDto, SetLoggerStdoutRequestDto, SetApmPriorityConfigurationRequestDto, SetStdoutPriorityConfigurationRequestDto};
use log::*;

#[derive(Clone)]
pub struct RuntimeConfigApiHandlerImpl {
    logger_guard: Arc<LogWorkerGuard>,
}

impl RuntimeConfigApiHandlerImpl {
    pub fn new(logger_guard: Arc<LogWorkerGuard>) -> Self {
        Self {
            logger_guard,
        }
    }
}

#[async_trait(?Send)]
impl RuntimeConfigApiHandler for RuntimeConfigApiHandlerImpl {

    async fn get_logger_configuration(&self) -> Result<LoggerConfigDto, ApiError> {
        trace!("RuntimeConfigApiHandlerImpl - get_logger_configuration");
        Ok(LoggerConfigDto{
            level: self.logger_guard.level(),
            apm_enabled: self.logger_guard.apm_enabled(),
            stdout_enabled: self.logger_guard.stdout_enabled()
        })
    }

    async fn set_logger_level(&self, logger_config: SetLoggerLevelRequestDto) -> Result<(), ApiError> {
        info!("RuntimeConfigApiHandlerImpl - set_logger_level to: [{}]", logger_config.level);
        self.logger_guard.set_level(&logger_config.level).map_err(|err| ApiError::BadRequestError { cause: format!("{:?}", err)})
    }

    async fn set_apm_enabled(
        &self,
        logger_config: SetLoggerApmRequestDto,
    ) -> () {
        info!("RuntimeConfigApiHandlerImpl - set_apm_enabled to: [{}]", logger_config.enabled);
        self.logger_guard.set_apm_enabled(logger_config.enabled)
    }

    async fn set_stdout_enabled(
        &self,
        logger_config: SetLoggerStdoutRequestDto,
    ) -> Result<(), ApiError> {
        info!("RuntimeConfigApiHandlerImpl - set_stdout_enabled to: [{}]", logger_config.enabled);
        self.logger_guard.set_stdout_enabled(logger_config.enabled);
        Ok(())
    }

    async fn set_apm_first_configuration(&self, dto: SetApmPriorityConfigurationRequestDto) -> Result<(), ApiError> {
        let logger_level = dto.logger_level.unwrap_or_else(|| "info,tornado=debug".to_owned());
        info!("RuntimeConfigApiHandlerImpl - set_apm_first_configuration with logger level [{}]", logger_level);
        self.logger_guard.set_apm_enabled(true);
        self.logger_guard.set_level(logger_level)
            .map_err(|err| ApiError::BadRequestError { cause: format!("{:?}", err)})?;
        self.logger_guard.set_stdout_enabled(false);
        Ok(())
    }

    async fn set_stdout_first_configuration(&self, _dto: SetStdoutPriorityConfigurationRequestDto) -> Result<(), ApiError> {
        info!("RuntimeConfigApiHandlerImpl - set_stdout_first_configuration");
        self.logger_guard.set_stdout_enabled(true);
        self.logger_guard.reset_level()
            .map(|_| self.logger_guard.set_apm_enabled(false))
            .map_err(|err| ApiError::BadRequestError { cause: format!("{:?}", err)})?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Arc;
    use tornado_common_logger::LoggerConfig;
    use tracing_subscriber::EnvFilter;
    use std::str::FromStr;
    use std::sync::atomic::AtomicBool;
    use tornado_common_logger::elastic_apm::ApmTracingConfig;

    #[actix_rt::test]
    async fn should_set_the_logger_level() {
        // Arrange
        let logger_level = "debug".to_owned();
        let config = LoggerConfig {
            file_output_path: None, 
            stdout_output: false,
            tracing_elastic_apm: ApmTracingConfig::default(),
            level: logger_level.clone()
        };
        let env_filter = EnvFilter::from_str(&logger_level).unwrap();

        let (_reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let log_guard = Arc::new(LogWorkerGuard::new(None,None, config.clone().into(), AtomicBool::new(true).into(), AtomicBool::new(false).into(),reloadable_env_filter_handle));

        let api = RuntimeConfigApiHandlerImpl::new(log_guard);

        // Act
        let logger_level_before = api.get_logger_configuration().await.unwrap();

        api.set_logger_level(SetLoggerLevelRequestDto{
            level: "info".to_owned()
        }).await.unwrap();

        let logger_level_after = api.get_logger_configuration().await.unwrap();

        // Assert
        assert_eq!(logger_level, logger_level_before.level);
        assert_eq!("info", &logger_level_after.level);
    }

    #[actix_rt::test]
    async fn should_enable_apm_logger() {
        // Arrange
        let logger_level = "debug".to_owned();
        let config = LoggerConfig {
            file_output_path: None, 
            stdout_output: false,
            tracing_elastic_apm: ApmTracingConfig::default(),
            level: logger_level.clone()
        };
        let env_filter = EnvFilter::from_str(&logger_level).unwrap();

        let (_reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let log_guard = Arc::new(LogWorkerGuard::new(None,None, config.clone().into(), AtomicBool::new(false).into(), AtomicBool::new(false).into(),reloadable_env_filter_handle));

        let api = RuntimeConfigApiHandlerImpl::new(log_guard.clone());

        // Act
        assert!(!log_guard.apm_enabled());

        api.set_apm_enabled(SetLoggerApmRequestDto{
            enabled: true
        }).await;

        // Assert
        assert!(log_guard.apm_enabled());
    }

    #[actix_rt::test]
    async fn should_enable_stdout_logger() {
        // Arrange
        let logger_level = "debug".to_owned();
        let config = LoggerConfig {
            file_output_path: None, 
            stdout_output: false,
            tracing_elastic_apm: ApmTracingConfig::default(),
            level: logger_level.clone()
        };
        let env_filter = EnvFilter::from_str(&logger_level).unwrap();

        let (_reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let log_guard = Arc::new(LogWorkerGuard::new(None,None, config.clone().into(), AtomicBool::new(false).into(), AtomicBool::new(false).into(),reloadable_env_filter_handle));

        let api = RuntimeConfigApiHandlerImpl::new(log_guard.clone());

        // Act
        assert!(!log_guard.stdout_enabled());

        api.set_stdout_enabled(SetLoggerStdoutRequestDto{
            enabled: true
        }).await.unwrap();

        // Assert
        assert!(log_guard.stdout_enabled());
    }

    #[actix_rt::test]
    async fn should_enable_apm_or_stdout_first_logger_config() {
        // Arrange
        let logger_level = "debug".to_owned();
        let config = LoggerConfig {
            file_output_path: None, 
            stdout_output: false,
            tracing_elastic_apm: ApmTracingConfig::default(),
            level: logger_level.clone()
        };
        let env_filter = EnvFilter::from_str(&logger_level).unwrap();

        let (_reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let log_guard = Arc::new(LogWorkerGuard::new(None,None, config.clone().into(), AtomicBool::new(true).into(), AtomicBool::new(false).into(),reloadable_env_filter_handle));

        let api = RuntimeConfigApiHandlerImpl::new(log_guard.clone());

        // Set APM first
        {
            // Act
            assert!(!log_guard.apm_enabled());
            assert!(log_guard.stdout_enabled());

            api.set_apm_first_configuration(SetApmPriorityConfigurationRequestDto {
                logger_level: None
            }).await.unwrap();

            // Assert
            assert!(log_guard.apm_enabled());
            assert!(!log_guard.stdout_enabled());
            assert_eq!("info,tornado=debug" ,&log_guard.level());
        }

        // Set stdout first
        {
            // Act
            api.set_stdout_first_configuration(SetStdoutPriorityConfigurationRequestDto {}).await.unwrap();

            // Assert
            assert!(!log_guard.apm_enabled());
            assert!(log_guard.stdout_enabled());
            assert_eq!(&logger_level ,&log_guard.level());
        }
    }
}