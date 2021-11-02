use crate::auth::{AuthContext, Permission};
use crate::error::ApiError;
use crate::event::api::{EventApiHandler, ProcessType, SendEventRequest};
use std::sync::Arc;
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
        auth: AuthContext<'_>,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError> {
        match event.process_type {
            ProcessType::Full => auth.has_permission(&Permission::EventsFullProcess)?,
            ProcessType::SkipActions => auth.has_any_permission(&[
                &Permission::ConfigEdit,
                &Permission::ConfigView,
                &Permission::EventsFullProcess,
            ])?,
        };

        self.handler.send_event_to_current_config(event).await
    }

    pub async fn send_event_to_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError> {
        let draft = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft)?;

        match event.process_type {
            ProcessType::Full => auth.has_permission(&Permission::EventsFullProcess)?,
            ProcessType::SkipActions => {
                auth.has_any_permission(&[&Permission::ConfigEdit, &Permission::EventsFullProcess])?
            }
        };

        self.handler.send_event_to_config(event, draft.config).await
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::auth::Permission;
    use crate::error::ApiError;
    use async_trait::async_trait;
    use std::collections::{BTreeMap, HashMap};
    use tornado_common_api::{Event, Value};
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_matcher::config::{
        MatcherConfig, MatcherConfigDraft, MatcherConfigDraftData,
    };
    use tornado_engine_matcher::error::MatcherError;
    use tornado_engine_matcher::model::{ProcessedNode, ProcessedRules};

    pub struct TestApiHandler {}

    #[async_trait(?Send)]
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

        async fn send_event_to_config(
            &self,
            event: SendEventRequest,
            _config: MatcherConfig,
        ) -> Result<ProcessedEvent, ApiError> {
            self.send_event_to_current_config(event).await
        }
    }

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

    pub struct TestConfigManager {}

    #[async_trait::async_trait(?Send)]
    impl MatcherConfigEditor for TestConfigManager {
        async fn get_drafts(&self) -> Result<Vec<String>, MatcherError> {
            Ok(vec![])
        }

        async fn get_draft(&self, draft_id: &str) -> Result<MatcherConfigDraft, MatcherError> {
            Ok(MatcherConfigDraft {
                data: MatcherConfigDraftData {
                    user: DRAFT_OWNER_ID.to_owned(),
                    draft_id: draft_id.to_owned(),
                    created_ts_ms: 0,
                    updated_ts_ms: 0,
                },
                config: MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![] },
            })
        }

        async fn create_draft(&self, _user: String) -> Result<String, MatcherError> {
            Ok("".to_owned())
        }

        async fn update_draft(
            &self,
            _draft_id: &str,
            _user: String,
            _config: &MatcherConfig,
        ) -> Result<(), MatcherError> {
            Ok(())
        }

        async fn deploy_draft(&self, _draft_id: &str) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Ruleset { name: "ruleset_new".to_owned(), rules: vec![] })
        }

        async fn delete_draft(&self, _draft_id: &str) -> Result<(), MatcherError> {
            Ok(())
        }

        async fn draft_take_over(
            &self,
            _draft_id: &str,
            _user: String,
        ) -> Result<(), MatcherError> {
            Ok(())
        }

        async fn deploy_config(
            &self,
            _config: &MatcherConfig,
        ) -> Result<MatcherConfig, MatcherError> {
            unimplemented!()
        }
    }

    #[actix_rt::test]
    async fn send_event_to_configuration_with_skip_action_should_require_edit_view_or_full_permission(
    ) {
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
        assert!(api.send_event_to_current_config(user_full_process, request.clone()).await.is_ok());
        assert!(api
            .send_event_to_current_config(user_no_permission, request.clone())
            .await
            .is_err());
    }

    #[actix_rt::test]
    async fn send_event_to_configuration_with_full_execution_should_require_events_full_process_permission(
    ) {
        // Arrange
        let api = EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions = auth_permissions();
        let (user_view, user_edit, user_full_process) = create_users(&permissions);

        let request =
            SendEventRequest { event: Event::new("event"), process_type: ProcessType::Full };

        // Act & Assert
        assert!(api.send_event_to_current_config(user_edit, request.clone()).await.is_err());
        assert!(api.send_event_to_current_config(user_view, request.clone()).await.is_err());
        assert!(api.send_event_to_current_config(user_full_process, request.clone()).await.is_ok());
    }

    #[actix_rt::test]
    async fn send_event_to_draft_with_skip_action_should_have_edit_or_full_process_permission_and_ownership(
    ) {
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
            .is_ok());
        assert!(api.send_event_to_draft(user_view.clone(), "id", request.clone()).await.is_err());
    }

    #[actix_rt::test]
    async fn send_event_to_draft_with_full_execution_should_have_full_process_permission_and_ownership(
    ) {
        // Arrange
        let api = EventApiV2::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();

        let (mut user_view, mut user_edit, mut user_full_process) = create_users(&permissions_map);

        let request = SendEventRequest {
            event: Event::new("event_for_draft"),
            process_type: ProcessType::Full,
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

        assert!(api
            .send_event_to_draft(user_full_process.clone(), "id", request.clone())
            .await
            .is_ok());
        assert!(api.send_event_to_draft(user_edit.clone(), "id", request.clone()).await.is_err());
        assert!(api.send_event_to_draft(user_view.clone(), "id", request.clone()).await.is_err());
    }
}
