use crate::auth::{AuthContext, Permission};
use crate::error::ApiError;
use crate::event::api::{EventApiHandler, ProcessType, SendEventRequest};
use std::sync::Arc;
use tornado_engine_matcher::config::MatcherConfigEditor;
use tornado_engine_matcher::model::ProcessedEvent;
use std::collections::HashMap;
use tornado_engine_matcher::config::fs::ROOT_NODE_NAME;
use tornado_engine_matcher::config::operation::NodeFilter;

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
        auth: AuthContext<'_>,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError> {
        auth.has_any_permission(&[&Permission::ConfigView, &Permission::ConfigEdit])?;
        match event.process_type {
            ProcessType::Full => {
                auth.has_permission(&Permission::EventsFullProcess)?;
            }
            ProcessType::SkipActions => {}
        };
        let config_filter = HashMap::from([
            (ROOT_NODE_NAME.to_owned(), NodeFilter::AllChildren)
        ]);

        self.handler.send_event_to_current_config(config_filter, event).await
    }

    pub async fn send_event_to_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError> {
        let draft = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft)?;

        auth.has_permission(&Permission::ConfigEdit)?;
        match event.process_type {
            ProcessType::Full => {
                auth.has_permission(&Permission::EventsFullProcess)?;
            }
            ProcessType::SkipActions => {}
        };

        self.handler.send_event_to_config(event, draft.config).await
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::auth::Permission;
    use std::collections::BTreeMap;
    use tornado_common_api::{Event};
    use tornado_engine_api_dto::auth::Auth;
    use crate::event::api::test::{TestApiHandler, TestConfigManager};

    fn auth_permissions() -> BTreeMap<Permission, Vec<String>> {
        let mut permission_roles_map = BTreeMap::new();
        permission_roles_map.insert(Permission::ConfigEdit, vec!["edit".to_owned()]);
        permission_roles_map.insert(Permission::ConfigView, vec!["view".to_owned()]);
        permission_roles_map
            .insert(Permission::EventsFullProcess, vec!["events_full_process".to_owned()]);
        permission_roles_map
    }

    fn create_users(
        permissions_map: &BTreeMap<Permission, Vec<String>>,
    ) -> (AuthContext, AuthContext, AuthContext) {
        let user_view = AuthContext::new(
            Auth { user: "user_id".to_owned(), roles: vec!["view".to_owned()], preferences: None },
            permissions_map,
        );

        let user_edit = AuthContext::new(
            Auth { user: "user_id".to_owned(), roles: vec!["edit".to_owned()], preferences: None },
            permissions_map,
        );

        let user_full_process = AuthContext::new(
            Auth {
                user: "user_id".to_owned(),
                roles: vec!["events_full_process".to_owned()],
                preferences: None,
            },
            permissions_map,
        );

        (user_view, user_edit, user_full_process)
    }

    pub const DRAFT_OWNER_ID: &str = "OWNER";

    #[actix_rt::test]
    async fn send_event_to_configuration_with_skip_action_should_require_edit_or_view_permission() {
        // Arrange
        let api = EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();

        let (user_view, user_edit, user_full_process) = create_users(&permissions_map);
        let user_no_permission = AuthContext::new(
            Auth { user: "user_id".to_owned(), roles: vec![], preferences: None },
            &permissions_map,
        );

        let request =
            SendEventRequest { event: Event::new("event"), process_type: ProcessType::SkipActions };

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
    async fn send_event_to_configuration_with_full_execution_should_require_events_full_process_and_view_or_edit_permission(
    ) {
        // Arrange
        let api = EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions = auth_permissions();
        let (user_view, user_edit, user_full_process) = create_users(&permissions);
        let user_view_and_full_process = AuthContext::new(
            Auth {
                user: "user_id".to_owned(),
                roles: vec!["events_full_process".to_owned(), "view".to_owned()],
                preferences: None,
            },
            &permissions,
        );
        let user_edit_and_full_process = AuthContext::new(
            Auth {
                user: "user_id".to_owned(),
                roles: vec!["events_full_process".to_owned(), "edit".to_owned()],
                preferences: None,
            },
            &permissions,
        );

        let request =
            SendEventRequest { event: Event::new("event"), process_type: ProcessType::Full };

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

    #[actix_rt::test]
    async fn send_event_to_draft_with_skip_action_should_have_edit_permission_and_ownership() {
        // Arrange
        let api = EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();

        let (mut user_view, mut user_edit, mut user_full_process) = create_users(&permissions_map);

        let request = SendEventRequest {
            event: Event::new("event_for_draft"),
            process_type: ProcessType::SkipActions,
        };

        // Act & Assert
        assert!(api.send_event_to_draft(user_edit.clone(), "id", request.clone()).await.is_err());
        assert!(api.send_event_to_draft(user_view.clone(), "id", request.clone()).await.is_err());
        assert!(api
            .send_event_to_draft(user_full_process.clone(), "id", request.clone())
            .await
            .is_err());

        // Set the users as owners of the draft
        user_edit.auth.user = DRAFT_OWNER_ID.to_owned();
        user_view.auth.user = DRAFT_OWNER_ID.to_owned();
        user_full_process.auth.user = DRAFT_OWNER_ID.to_owned();

        assert!(api.send_event_to_draft(user_edit.clone(), "id", request.clone()).await.is_ok());
        assert!(api
            .send_event_to_draft(user_full_process.clone(), "id", request.clone())
            .await
            .is_err());
        assert!(api.send_event_to_draft(user_view.clone(), "id", request.clone()).await.is_err());
    }

    #[actix_rt::test]
    async fn send_event_to_draft_with_full_execution_should_have_full_process_and_edit_permission_and_ownership(
    ) {
        // Arrange
        let api = EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();

        let (mut user_view, mut user_edit, mut user_full_process) = create_users(&permissions_map);

        let request = SendEventRequest {
            event: Event::new("event_for_draft"),
            process_type: ProcessType::Full,
        };
        let mut user_view_and_full_process = AuthContext::new(
            Auth {
                user: "user_id".to_owned(),
                roles: vec!["events_full_process".to_owned(), "view".to_owned()],
                preferences: None,
            },
            &permissions_map,
        );
        let mut user_edit_and_full_process = AuthContext::new(
            Auth {
                user: "user_id".to_owned(),
                roles: vec!["events_full_process".to_owned(), "edit".to_owned()],
                preferences: None,
            },
            &permissions_map,
        );

        // Act & Assert
        assert!(api.send_event_to_draft(user_edit.clone(), "id", request.clone()).await.is_err());
        assert!(api.send_event_to_draft(user_view.clone(), "id", request.clone()).await.is_err());
        assert!(api
            .send_event_to_draft(user_full_process.clone(), "id", request.clone())
            .await
            .is_err());
        assert!(api
            .send_event_to_draft(user_view_and_full_process.clone(), "id", request.clone())
            .await
            .is_err());
        assert!(api
            .send_event_to_draft(user_edit_and_full_process.clone(), "id", request.clone())
            .await
            .is_err());

        // Set the users as owners of the draft
        user_edit.auth.user = DRAFT_OWNER_ID.to_owned();
        user_view.auth.user = DRAFT_OWNER_ID.to_owned();
        user_full_process.auth.user = DRAFT_OWNER_ID.to_owned();
        user_view_and_full_process.auth.user = DRAFT_OWNER_ID.to_owned();
        user_edit_and_full_process.auth.user = DRAFT_OWNER_ID.to_owned();

        assert!(api
            .send_event_to_draft(user_full_process.clone(), "id", request.clone())
            .await
            .is_err());
        assert!(api.send_event_to_draft(user_edit.clone(), "id", request.clone()).await.is_err());
        assert!(api.send_event_to_draft(user_view.clone(), "id", request.clone()).await.is_err());
        assert!(api
            .send_event_to_draft(user_view_and_full_process.clone(), "id", request.clone())
            .await
            .is_err());
        assert!(api
            .send_event_to_draft(user_edit_and_full_process.clone(), "id", request.clone())
            .await
            .is_ok());
    }
}
