use crate::auth::auth_v2::AuthContextV2;
use crate::auth::Permission;
use crate::error::ApiError;
use crate::event::api::{EventApiHandler, ProcessType, SendEventRequest};
use std::sync::Arc;
use tornado_engine_matcher::config::operation::NodeFilter;
use tornado_engine_matcher::config::MatcherConfigEditor;
use tornado_engine_matcher::model::ProcessedEvent;

pub struct EventApiV2<A: EventApiHandler, CM: MatcherConfigEditor> {
    handler: A,
    config_manager: Arc<CM>,
}

impl<A: EventApiHandler, CM: MatcherConfigEditor> EventApiV2<A, CM> {
    pub fn new(handler: A, config_manager: Arc<CM>) -> Self {
        Self { handler, config_manager }
    }

    /// Executes an event on the current Tornado configuration
    pub async fn send_event_to_current_config(
        &self,
        auth: AuthContextV2<'_>,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError> {
        auth.has_any_permission(&[&Permission::ConfigView, &Permission::ConfigEdit])?;
        match event.process_type {
            ProcessType::Full => {
                auth.has_permission(&Permission::TestEventExecuteActions)?;
            }
            ProcessType::SkipActions => {}
        };
        let config_filter = NodeFilter::map_from(&[auth.auth.authorization.path.clone()]);

        self.handler.send_event_to_current_config(config_filter, event).await
    }

}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::auth::Permission;
    use crate::event::api::test::{TestApiHandler, TestConfigManager};
    use std::collections::{BTreeMap};
    use tornado_common_api::{Event, Value};
    use tornado_engine_api_dto::auth_v2::{AuthV2, Authorization};

    fn auth_permissions() -> BTreeMap<Permission, Vec<String>> {
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigEdit, vec!["edit".to_owned()]);
        permission_roles_map.insert(Permission::ConfigView, vec!["view".to_owned()]);
        permission_roles_map.insert(
            Permission::TestEventExecuteActions,
            vec!["test_event_execute_actions".to_owned()],
        );
        permission_roles_map
    }

    fn create_users(
        permissions_map: &BTreeMap<Permission, Vec<String>>,
    ) -> (AuthContextV2, AuthContextV2, AuthContextV2) {

        let owner_view = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["view".to_owned()],
                },
                preferences: None,
            },
            permissions_map,
        );

        let owner_edit = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["edit".to_owned()],
                },
                preferences: None,
            },
            permissions_map,
        );

        let owner_full_process = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["test_event_execute_actions".to_owned()],
                },
                preferences: None,
            },
            permissions_map,
        );

        (owner_view, owner_edit, owner_full_process)
    }

    pub const DRAFT_OWNER_ID: &str = "OWNER";

    #[actix_rt::test]
    async fn send_event_to_configuration_with_skip_action_should_require_edit_or_view_permission() {
        // Arrange
        let api = EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();

        let (user_view, user_edit, user_full_process) = create_users(&permissions_map);
        let user_no_permission = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec![],
                },
                preferences: None,
            },
            &permissions_map,
        );

        let request =
            SendEventRequest { event: Event::new("event"), metadata: Value::Map(Default::default()), process_type: ProcessType::SkipActions };

        // Act & Assert
        assert!(api.send_event_to_current_config(user_edit, request.clone()).await.is_ok());
        assert!(api.send_event_to_current_config(user_view, request.clone()).await.is_ok());
        assert!(api
            .send_event_to_current_config(user_full_process, request.clone())
            .await
            .is_err());
        assert!(api
            .send_event_to_current_config(user_no_permission, request.clone())
            .await
            .is_err());
    }

    #[actix_rt::test]
    async fn send_event_to_configuration_with_full_execution_should_require_test_event_execute_actions_and_view_or_edit_permission(
    ) {
        // Arrange
        let api = EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions = auth_permissions();
        let (user_view, user_edit, user_full_process) = create_users(&permissions);
        let user_view_and_full_process = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["test_event_execute_actions".to_owned(), "view".to_owned()],
                },
                preferences: None,
            },
            &permissions,
        ); 
        let user_edit_and_full_process = AuthContextV2::new(
            AuthV2 {
                user: DRAFT_OWNER_ID.to_owned(),
                authorization: Authorization {
                    path: vec!["root".to_owned()],
                    roles: vec!["test_event_execute_actions".to_owned(), "edit".to_owned()],
                },
                preferences: None,
            },
            &permissions,
        );

        let request =
            SendEventRequest { event: Event::new("event"), metadata: Value::Map(Default::default()), process_type: ProcessType::Full };

        // Act & Assert
        assert!(api.send_event_to_current_config(user_edit, request.clone()).await.is_err());
        assert!(api.send_event_to_current_config(user_view, request.clone()).await.is_err());
        assert!(api
            .send_event_to_current_config(user_full_process, request.clone())
            .await
            .is_err());
        assert!(api
            .send_event_to_current_config(user_view_and_full_process, request.clone())
            .await
            .is_ok());
        assert!(api
            .send_event_to_current_config(user_edit_and_full_process, request.clone())
            .await
            .is_ok());
    }

}
