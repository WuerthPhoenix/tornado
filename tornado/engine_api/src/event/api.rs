use crate::auth::{AuthContext, Permission};
use crate::error::ApiError;
use async_trait::async_trait;
use tornado_common_api::Event;
use tornado_engine_matcher::model::ProcessedEvent;

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
#[async_trait]
pub trait EventApiHandler: Send + Sync {
    async fn send_event_to_current_config(
        &self,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError>;
}

#[derive(Clone)]
pub struct SendEventRequest {
    pub event: Event,
    pub process_type: ProcessType,
}

#[derive(Clone)]
pub enum ProcessType {
    Full,
    SkipActions,
}

pub struct EventApi<A: EventApiHandler> {
    handler: A,
}

impl<A: EventApiHandler> EventApi<A> {
    pub fn new(handler: A) -> Self {
        Self { handler }
    }

    /// Executes an event on the current Tornado configuration
    pub async fn send_event_to_current_config(
        &self,
        auth: AuthContext<'_>,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        self.handler.send_event_to_current_config(event).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::auth::Permission;
    use crate::error::ApiError;
    use async_trait::async_trait;
    use std::collections::{BTreeMap, HashMap};
    use tornado_common_api::Value;
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_matcher::model::{ProcessedNode, ProcessedRules};

    struct TestApiHandler {}

    #[async_trait]
    impl EventApiHandler for TestApiHandler {
        async fn send_event_to_current_config(
            &self,
            event: SendEventRequest,
        ) -> Result<ProcessedEvent, ApiError> {
            Ok(ProcessedEvent {
                event: event.event.into(),
                result: ProcessedNode::Ruleset {
                    name: "ruleset".to_owned(),
                    rules: ProcessedRules {
                        rules: vec![],
                        extracted_vars: Value::Map(HashMap::new()),
                    },
                },
            })
        }
    }

    fn auth_permissions() -> BTreeMap<Permission, Vec<String>> {
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigEdit, vec!["edit".to_owned()]);
        permission_roles_map.insert(Permission::ConfigView, vec!["view".to_owned()]);
        permission_roles_map
    }

    fn create_users(
        permissions_map: &BTreeMap<Permission, Vec<String>>,
    ) -> (AuthContext, AuthContext) {
        let user_view = AuthContext::new(
            Auth { user: "user_id".to_owned(), roles: vec!["view".to_owned()], preferences: None },
            permissions_map,
        );

        let user_edit = AuthContext::new(
            Auth { user: "user_id".to_owned(), roles: vec!["edit".to_owned()], preferences: None },
            permissions_map,
        );

        (user_view, user_edit)
    }

    #[actix_rt::test]
    async fn send_event_to_configuration_should_require_edit_permission() {
        // Arrange
        let api = EventApi::new(TestApiHandler {});
        let permissions_map = auth_permissions();

        let (user_view, user_edit) = create_users(&permissions_map);

        let request =
            SendEventRequest { event: Event::new("event"), process_type: ProcessType::Full };

        // Act & Assert
        assert!(api.send_event_to_current_config(user_edit, request.clone()).await.is_ok());
        assert!(api.send_event_to_current_config(user_view, request.clone()).await.is_err());
    }
}
