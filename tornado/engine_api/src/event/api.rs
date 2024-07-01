use crate::error::ApiError;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use tornado_common_api::Event;
use tornado_engine_matcher::config::operation::NodeFilter;
use tornado_engine_matcher::config::MatcherConfig;
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

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::auth::Permission;
    use crate::error::ApiError;
    use async_trait::async_trait;
    use std::collections::{BTreeMap, HashMap};
    use tornado_common_api::{Map, Value, WithEventData};
    use tornado_engine_api_dto::auth::Auth;
    use tornado_engine_matcher::config::nodes::Filter;
    use tornado_engine_matcher::config::{
        Defaultable, MatcherConfigDraft, MatcherConfigDraftData, MatcherConfigEditor,
        MatcherConfigReader,
    };
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

    pub const DRAFT_OWNER_ID: &str = "OWNER";

    pub struct TestConfigManager {}

    #[async_trait::async_trait(?Send)]
    impl MatcherConfigReader for TestConfigManager {
        async fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
            Ok(MatcherConfig::Filter {
                name: "root".to_owned(),
                filter: Filter {
                    description: "".to_string(),
                    active: true,
                    filter: Defaultable::Default {},
                },
                nodes: vec![MatcherConfig::Ruleset { name: "ruleset".to_owned(), rules: vec![] }],
            })
        }
    }

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
}
