use crate::auth::{AuthContext, Permission};
use crate::error::ApiError;
use tornado_engine_api_dto::runtime_config::{
    LoggerConfigDto, SetApmPriorityConfigurationRequestDto, SetLoggerApmRequestDto,
    SetLoggerLevelRequestDto, SetLoggerStdoutRequestDto, SetStdoutPriorityConfigurationRequestDto,
};

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
#[async_trait::async_trait(?Send)]
pub trait RuntimeConfigApiHandler: Send + Sync {
    async fn get_logger_configuration(&self) -> Result<LoggerConfigDto, ApiError>;

    async fn set_logger_level(
        &self,
        logger_config: SetLoggerLevelRequestDto,
    ) -> Result<(), ApiError>;

    async fn set_apm_enabled(&self, logger_config: SetLoggerApmRequestDto) -> ();

    async fn set_stdout_enabled(
        &self,
        logger_config: SetLoggerStdoutRequestDto,
    ) -> Result<(), ApiError>;

    async fn set_apm_first_configuration(
        &self,
        dto: SetApmPriorityConfigurationRequestDto,
    ) -> Result<(), ApiError>;

    async fn set_stdout_first_configuration(
        &self,
        dto: SetStdoutPriorityConfigurationRequestDto,
    ) -> Result<(), ApiError>;
}

pub struct RuntimeConfigApi<A: RuntimeConfigApiHandler> {
    handler: A,
}

impl<A: RuntimeConfigApiHandler> RuntimeConfigApi<A> {
    pub fn new(handler: A) -> Self {
        Self { handler }
    }

    /// Returns the current logger configuration of tornado
    pub async fn get_logger_configuration(
        &self,
        auth: AuthContext<'_>,
    ) -> Result<LoggerConfigDto, ApiError> {
        auth.has_permission(&Permission::RuntimeConfigView)?;
        self.handler.get_logger_configuration().await
    }

    /// Sets the current logger configuration of tornado
    pub async fn set_logger_level(
        &self,
        auth: AuthContext<'_>,
        logger_config: SetLoggerLevelRequestDto,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::RuntimeConfigEdit)?;
        self.handler.set_logger_level(logger_config).await
    }

    /// Enable or disable logging to apm
    pub async fn set_apm_enabled(
        &self,
        auth: AuthContext<'_>,
        dto: SetLoggerApmRequestDto,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::RuntimeConfigEdit)?;
        self.handler.set_apm_enabled(dto).await;
        Ok(())
    }

    /// Enable or disable logging to stdout
    pub async fn set_stdout_enabled(
        &self,
        auth: AuthContext<'_>,
        dto: SetLoggerStdoutRequestDto,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::RuntimeConfigEdit)?;
        self.handler.set_stdout_enabled(dto).await
    }

    /// Enable APM and disable the stdout.
    /// It sets also the logger level to DEBUG
    pub async fn set_apm_priority_configuration(
        &self,
        auth: AuthContext<'_>,
        dto: SetApmPriorityConfigurationRequestDto,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::RuntimeConfigEdit)?;
        self.handler.set_apm_first_configuration(dto).await
    }

    /// Enable the stdout and disable APM.
    /// It also reset the logger to the original configuration
    pub async fn set_stdout_priority_configuration(
        &self,
        auth: AuthContext<'_>,
        dto: SetStdoutPriorityConfigurationRequestDto,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::RuntimeConfigEdit)?;
        self.handler.set_stdout_first_configuration(dto).await
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::auth::Permission;
    use crate::error::ApiError;
    use std::collections::BTreeMap;
    use tornado_engine_api_dto::auth::Auth;

    pub struct TestRuntimeConfigApiHandler {}

    #[async_trait::async_trait(?Send)]
    impl RuntimeConfigApiHandler for TestRuntimeConfigApiHandler {
        async fn get_logger_configuration(&self) -> Result<LoggerConfigDto, ApiError> {
            Ok(LoggerConfigDto {
                level: "debug".to_owned(),
                apm_enabled: true,
                stdout_enabled: false,
            })
        }
        async fn set_logger_level(
            &self,
            _logger_config: SetLoggerLevelRequestDto,
        ) -> Result<(), ApiError> {
            Ok(())
        }

        async fn set_apm_enabled(&self, _logger_config: SetLoggerApmRequestDto) -> () {
            
        }

        async fn set_stdout_enabled(
            &self,
            _logger_config: SetLoggerStdoutRequestDto,
        ) -> Result<(), ApiError> {
            Ok(())
        }

        async fn set_apm_first_configuration(
            &self,
            _dto: SetApmPriorityConfigurationRequestDto,
        ) -> Result<(), ApiError> {
            Ok(())
        }

        async fn set_stdout_first_configuration(
            &self,
            _dto: SetStdoutPriorityConfigurationRequestDto,
        ) -> Result<(), ApiError> {
            Ok(())
        }
    }

    fn auth_permissions() -> BTreeMap<Permission, Vec<String>> {
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::RuntimeConfigEdit, vec!["edit".to_owned()]);
        permission_roles_map.insert(Permission::RuntimeConfigView, vec!["view".to_owned()]);
        permission_roles_map
    }

    #[actix_rt::test]
    async fn get_current_logger_configuration_should_require_view_permission() {
        // Arrange
        let api = RuntimeConfigApi::new(TestRuntimeConfigApiHandler {});
        let permissions_map = &auth_permissions();

        let auth_view = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["view".to_owned()], preferences: None },
            permissions_map,
        );

        let auth_edit = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["edit".to_owned()], preferences: None },
            permissions_map,
        );

        // Act & Assert
        assert!(api.get_logger_configuration(auth_view).await.is_ok());
        assert!(api.get_logger_configuration(auth_edit).await.is_err());
    }

    #[actix_rt::test]
    async fn set_current_logger_level_should_require_edit_permission() {
        // Arrange
        let api = RuntimeConfigApi::new(TestRuntimeConfigApiHandler {});
        let permissions_map = &auth_permissions();

        let auth_view = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["view".to_owned()], preferences: None },
            permissions_map,
        );

        let auth_edit = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["edit".to_owned()], preferences: None },
            permissions_map,
        );

        let logger_config = SetLoggerLevelRequestDto { level: "".to_owned() };

        // Act & Assert
        assert!(api.set_logger_level(auth_view, logger_config.clone()).await.is_err());
        assert!(api.set_logger_level(auth_edit, logger_config).await.is_ok());
    }

    #[actix_rt::test]
    async fn set_apm_configuration_should_require_edit_permission() {
        // Arrange
        let api = RuntimeConfigApi::new(TestRuntimeConfigApiHandler {});
        let permissions_map = &auth_permissions();

        let auth_view = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["view".to_owned()], preferences: None },
            permissions_map,
        );

        let auth_edit = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["edit".to_owned()], preferences: None },
            permissions_map,
        );

        let dto = SetLoggerApmRequestDto { enabled: false };

        // Act & Assert
        assert!(api.set_apm_enabled(auth_view, dto.clone()).await.is_err());
        assert!(api.set_apm_enabled(auth_edit, dto).await.is_ok());
    }

    #[actix_rt::test]
    async fn set_stdout_configuration_should_require_edit_permission() {
        // Arrange
        let api = RuntimeConfigApi::new(TestRuntimeConfigApiHandler {});
        let permissions_map = &auth_permissions();

        let auth_view = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["view".to_owned()], preferences: None },
            permissions_map,
        );

        let auth_edit = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["edit".to_owned()], preferences: None },
            permissions_map,
        );

        let dto = SetLoggerStdoutRequestDto { enabled: false };

        // Act & Assert
        assert!(api.set_stdout_enabled(auth_view, dto.clone()).await.is_err());
        assert!(api.set_stdout_enabled(auth_edit, dto).await.is_ok());
    }

    #[actix_rt::test]
    async fn set_apm_first_configuration_should_require_edit_permission() {
        // Arrange
        let api = RuntimeConfigApi::new(TestRuntimeConfigApiHandler {});
        let permissions_map = &auth_permissions();

        let auth_view = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["view".to_owned()], preferences: None },
            permissions_map,
        );

        let auth_edit = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["edit".to_owned()], preferences: None },
            permissions_map,
        );

        let dto = SetApmPriorityConfigurationRequestDto { logger_level: None };

        // Act & Assert
        assert!(api.set_apm_priority_configuration(auth_view, dto.clone()).await.is_err());
        assert!(api.set_apm_priority_configuration(auth_edit, dto).await.is_ok());
    }

    #[actix_rt::test]
    async fn set_stdout_first_configuration_should_require_edit_permission() {
        // Arrange
        let api = RuntimeConfigApi::new(TestRuntimeConfigApiHandler {});
        let permissions_map = &auth_permissions();

        let auth_view = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["view".to_owned()], preferences: None },
            permissions_map,
        );

        let auth_edit = AuthContext::new(
            Auth { user: "1".to_owned(), roles: vec!["edit".to_owned()], preferences: None },
            permissions_map,
        );

        let dto = SetStdoutPriorityConfigurationRequestDto {};

        // Act & Assert
        assert!(api.set_stdout_priority_configuration(auth_view, dto.clone()).await.is_err());
        assert!(api.set_stdout_priority_configuration(auth_edit, dto).await.is_ok());
    }
}
