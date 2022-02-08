use crate::auth::{AuthContext, AuthContextTrait, Permission};
use crate::error::ApiError;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tornado_common_api::Event;
use tornado_engine_matcher::config::fs::ROOT_NODE_NAME;
use tornado_engine_matcher::config::operation::NodeFilter;
use tornado_engine_matcher::config::{MatcherConfig, MatcherConfigEditor};
use tornado_engine_matcher::model::ProcessedEvent;

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
#[async_trait(?Send)]
pub trait EventApiHandler: Send + Sync {
    /// Executes an Event on the current Tornado Configuration
    async fn send_event_to_current_config(
        &self,
        config_filter: HashMap<String, NodeFilter>,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError>;

    /// Executes an Event on a custom Tornado Configuration
    async fn send_event_to_config(
        &self,
        event: SendEventRequest,
        config: MatcherConfig,
    ) -> Result<ProcessedEvent, ApiError>;
}

#[derive(Clone)]
pub struct SendEventRequest {
    pub event: Event,
    pub process_type: ProcessType,
}

impl SendEventRequest {
    pub fn to_event_with_metadata(&self) -> Value {
        json!(&self.event)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessType {
    Full,
    SkipActions,
}

pub struct EventApi<A: EventApiHandler, CM: MatcherConfigEditor> {
    handler: A,
    config_manager: Arc<CM>,
}

impl<A: EventApiHandler, CM: MatcherConfigEditor> EventApi<A, CM> {
    pub fn new(handler: A, config_manager: Arc<CM>) -> Self {
        Self { handler, config_manager }
    }

    /// Executes an event on the current Tornado configuration
    pub async fn send_event_to_current_config(
        &self,
        auth: AuthContext<'_>,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let config_filter = HashMap::from([(ROOT_NODE_NAME.to_owned(), NodeFilter::AllChildren)]);
        self.handler.send_event_to_current_config(config_filter, event).await
    }

    pub async fn send_event_to_draft(
        &self,
        auth: AuthContext<'_>,
        draft_id: &str,
        event: SendEventRequest,
    ) -> Result<ProcessedEvent, ApiError> {
        auth.has_permission(&Permission::ConfigEdit)?;
        let draft = self.config_manager.get_draft(draft_id).await?;
        auth.is_owner(&draft)?;
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
    use tornado_common_api::{Map, Value, WithEventData};
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_matcher::config::filter::Filter;
    use tornado_engine_matcher::config::{Defaultable, MatcherConfigDraft, MatcherConfigDraftData};
    use tornado_engine_matcher::error::MatcherError;
    use tornado_engine_matcher::model::{ProcessedNode, ProcessedRules};

    pub struct TestApiHandler {}

    #[async_trait(?Send)]
    impl EventApiHandler for TestApiHandler {
        async fn send_event_to_current_config(
            &self,
            _config_filter: HashMap<String, NodeFilter>,
            event: SendEventRequest,
        ) -> Result<ProcessedEvent, ApiError> {
            Ok(ProcessedEvent {
                event: event.to_event_with_metadata(),
                result: ProcessedNode::Ruleset {
                    name: "ruleset".to_owned(),
                    rules: ProcessedRules {
                        rules: vec![],
                        extracted_vars: Value::Object(Map::new()),
                    },
                },
            })
        }

        async fn send_event_to_config(
            &self,
            event: SendEventRequest,
            _config: MatcherConfig,
        ) -> Result<ProcessedEvent, ApiError> {
            Ok(ProcessedEvent {
                event: event.to_event_with_metadata(),
                result: ProcessedNode::Ruleset {
                    name: "ruleset".to_owned(),
                    rules: ProcessedRules {
                        rules: vec![],
                        extracted_vars: Value::Object(Map::new()),
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
                config: MatcherConfig::Filter {
                    name: "root".to_owned(),
                    filter: Filter {
                        description: "".to_string(),
                        active: true,
                        filter: Defaultable::Default {},
                    },
                    nodes: vec![MatcherConfig::Ruleset {
                        name: "ruleset".to_owned(),
                        rules: vec![],
                    }],
                },
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
    async fn send_event_to_configuration_should_require_edit_permission() {
        // Arrange
        let api = EventApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();

        let (user_view, user_edit) = create_users(&permissions_map);

        let request =
            SendEventRequest { event: Event::new("event"), process_type: ProcessType::Full };

        // Act & Assert
        assert!(api.send_event_to_current_config(user_edit, request.clone()).await.is_ok());
        assert!(api.send_event_to_current_config(user_view, request.clone()).await.is_err());
    }

    #[actix_rt::test]
    async fn send_event_to_draft_should_require_owner_and_edit_permission() {
        // Arrange
        let api = EventApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();

        let (mut user_view, mut user_edit) = create_users(&permissions_map);

        let request = SendEventRequest {
            event: Event::new("event_for_draft"),
            process_type: ProcessType::SkipActions,
        };

        // Act & Assert
        assert!(api.send_event_to_draft(user_edit.clone(), "id", request.clone()).await.is_err());
        assert!(api.send_event_to_draft(user_view.clone(), "id", request.clone()).await.is_err());

        // Set the users as owners of the draft
        user_edit.auth.user = DRAFT_OWNER_ID.to_owned();
        user_view.auth.user = DRAFT_OWNER_ID.to_owned();
        assert!(api.send_event_to_draft(user_edit.clone(), "id", request.clone()).await.is_ok());
        assert!(api.send_event_to_draft(user_view.clone(), "id", request.clone()).await.is_err());
    }

    #[actix_rt::test]
    async fn send_event_to_current_config_should_propagate_metadata() {
        // Arrange
        let api = EventApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();

        let (_user_view, user_edit) = create_users(&permissions_map);

        let mut event = Event::new("event");
        let mut metadata = Map::new();
        metadata
            .insert("something".to_owned(), Value::String(format!("{}", rand::random::<usize>())));
        event.metadata = metadata.clone();

        let request = SendEventRequest { event, process_type: ProcessType::SkipActions };

        // Act
        let result = api.send_event_to_current_config(user_edit, request.clone()).await.unwrap();

        // Assert
        assert_eq!(&serde_json::to_value(&metadata).unwrap(), result.event.metadata().unwrap());
    }

    #[actix_rt::test]
    async fn send_event_to_draft_should_propagate_metadata() {
        // Arrange
        let api = EventApi::new(TestApiHandler {}, Arc::new(TestConfigManager {}));
        let permissions_map = auth_permissions();

        let (_user_view, mut user_edit) = create_users(&permissions_map);
        user_edit.auth.user = DRAFT_OWNER_ID.to_owned();

        let mut event = Event::new("event");
        let mut metadata = Map::new();
        metadata
            .insert("something".to_owned(), Value::String(format!("{}", rand::random::<usize>())));
        event.metadata = metadata.clone();

        let request = SendEventRequest { event, process_type: ProcessType::SkipActions };

        // Act
        let result = api.send_event_to_draft(user_edit, "id", request.clone()).await.unwrap();

        // Assert
        assert_eq!(&serde_json::to_value(&metadata).unwrap(), result.event.metadata().unwrap());
    }
}
