use async_trait::async_trait;
use tornado_engine_api::error::ApiError;
use tornado_engine_api::runtime_config::api::RuntimeConfigApiHandler;
use std::sync::Arc;
use tornado_common_logger::LogWorkerGuard;
use tornado_engine_api_dto::runtime_config::{LoggerConfigDto, SetLoggerApmRequestDto, SetLoggerLevelRequestDto, SetLoggerStdoutRequestDto};
use tokio::sync::RwLock;
use log::*;

#[derive(Clone)]
pub struct RuntimeConfigApiHandlerImpl {
    logger_guard: Arc<LogWorkerGuard>,
    #[deprecated]
    logger_level: Arc<RwLock<String>>
}

impl RuntimeConfigApiHandlerImpl {
    pub fn new(logger_guard: Arc<LogWorkerGuard>, logger_level: Arc<RwLock<String>>) -> Self {
        Self {
            logger_guard,
            logger_level
        }
    }
}

#[async_trait(?Send)]
impl RuntimeConfigApiHandler for RuntimeConfigApiHandlerImpl {

    async fn get_logger_configuration(&self) -> Result<LoggerConfigDto, ApiError> {
        let logger_level_guard = self.logger_level.read().await;
        trace!("RuntimeConfigApiHandlerImpl - get_logger_configuration: [{}]", *logger_level_guard);
        Ok(LoggerConfigDto{
            level: logger_level_guard.clone(),
            apm_enabled: self.logger_guard.apm_enabled(),
            stdout_enabled: self.logger_guard.stdout_enabled()
        })
    }

    async fn set_logger_level(&self, logger_config: SetLoggerLevelRequestDto) -> Result<(), ApiError> {
        info!("RuntimeConfigApiHandlerImpl - set_logger_level to: [{}]", logger_config.level);
        self.logger_guard.set_level(&logger_config.level).map_err(|err| ApiError::BadRequestError { cause: format!("{:?}", err)})?;
        let mut logger_level_guard = self.logger_level.write().await;
        *logger_level_guard = logger_config.level;
        Ok(())
    }

    async fn set_apm_enabled(
        &self,
        logger_config: SetLoggerApmRequestDto,
    ) -> Result<(), ApiError> {
        self.logger_guard.set_apm_enabled(logger_config.enabled).map_err(|err| ApiError::BadRequestError { cause: format!("{:?}", err)})
    }

    async fn set_stdout_enabled(
        &self,
        logger_config: SetLoggerStdoutRequestDto,
    ) -> Result<(), ApiError> {
        self.logger_guard.set_stdout_enabled(logger_config.enabled);
        Ok(())
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Arc;
    use tracing_subscriber::EnvFilter;
    use std::str::FromStr;
    use std::sync::atomic::AtomicBool;

    #[actix_rt::test]
    async fn should_set_the_logger_level() {
        // Arrange
        let logger_level = "debug".to_owned();
        let env_filter = EnvFilter::from_str(&logger_level).unwrap();

        let (_reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let log_guard = Arc::new(LogWorkerGuard::new(None,None, logger_level.clone(), AtomicBool::new(true).into(), None,reloadable_env_filter_handle));

        let api = RuntimeConfigApiHandlerImpl::new(log_guard, Arc::new(RwLock::new(logger_level.clone())));

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
        let env_filter = EnvFilter::from_str(&logger_level).unwrap();

        let (_reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let log_guard = Arc::new(LogWorkerGuard::new(None,None, logger_level.clone(), AtomicBool::new(false).into(), Some(AtomicBool::new(false).into()),reloadable_env_filter_handle));

        let api = RuntimeConfigApiHandlerImpl::new(log_guard.clone(), Arc::new(RwLock::new(logger_level.clone())));

        // Act
        assert!(!log_guard.apm_enabled());

        api.set_apm_enabled(SetLoggerApmRequestDto{
            enabled: true
        }).await.unwrap();

        // Assert
        assert!(log_guard.apm_enabled());
    }

    #[actix_rt::test]
    async fn should_enable_stdout_logger() {
        // Arrange
        let logger_level = "debug".to_owned();
        let env_filter = EnvFilter::from_str(&logger_level).unwrap();

        let (_reloadable_env_filter, reloadable_env_filter_handle) =
            tracing_subscriber::reload::Layer::new(env_filter);

        let log_guard = Arc::new(LogWorkerGuard::new(None,None, logger_level.clone(), AtomicBool::new(false).into(), Some(AtomicBool::new(false).into()),reloadable_env_filter_handle));

        let api = RuntimeConfigApiHandlerImpl::new(log_guard.clone(), Arc::new(RwLock::new(logger_level.clone())));

        // Act
        assert!(!log_guard.stdout_enabled());

        api.set_stdout_enabled(SetLoggerStdoutRequestDto{
            enabled: true
        }).await.unwrap();

        // Assert
        assert!(log_guard.stdout_enabled());
    }
}