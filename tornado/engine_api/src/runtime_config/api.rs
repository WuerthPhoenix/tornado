use crate::auth::{AuthContext, Permission};
use crate::error::ApiError;
use tornado_engine_api_dto::runtime_config::LoggerConfigDto;

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
#[async_trait::async_trait(?Send)]
pub trait RuntimeConfigApiHandler: Send + Sync {
    async fn get_logger_configuration(&self) -> Result<LoggerConfigDto, ApiError>;
    async fn set_logger_configuration(
        &self,
        logger_config: LoggerConfigDto,
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
    pub async fn set_logger_configuration(
        &self,
        auth: AuthContext<'_>,
        logger_config: LoggerConfigDto,
    ) -> Result<(), ApiError> {
        auth.has_permission(&Permission::RuntimeConfigEdit)?;
        self.handler.set_logger_configuration(logger_config).await
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
            Ok(LoggerConfigDto { level: "debug".to_owned() })
        }
        async fn set_logger_configuration(
            &self,
            _logger_config: LoggerConfigDto,
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
    async fn set_current_logger_configuration_should_require_edit_permission() {
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

        let logger_config = LoggerConfigDto { level: "".to_owned() };

        // Act & Assert
        assert!(api.set_logger_configuration(auth_view, logger_config.clone()).await.is_err());
        assert!(api.set_logger_configuration(auth_edit, logger_config).await.is_ok());
    }
}
