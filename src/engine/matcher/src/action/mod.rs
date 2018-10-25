use accessor::{Accessor, AccessorBuilder};
use config::Action as ConfigAction;
use error::MatcherError;
use model::ProcessedEvent;
use std::collections::HashMap;
use tornado_common_api::Action;

pub struct MatcherActionBuilder{
    accessor: AccessorBuilder,
}

impl MatcherActionBuilder {

    pub fn new() -> MatcherActionBuilder {
        MatcherActionBuilder{
            accessor: AccessorBuilder::new(),
        }
    }

    pub fn build(&self, rule_name: &str, actions: &[ConfigAction]) -> Result<Vec<MatcherAction>, MatcherError> {
        let mut matcher_actions = vec![];

        for action in actions {
            let mut matcher_action = MatcherAction{
                id: action.id.to_owned(),
                payload: HashMap::new()
            };

            for (payload_key, payload_value) in action.payload {
                matcher_action.payload.insert(
                    payload_key.to_owned(),
                    self.accessor.build(rule_name, &payload_value)?
                );
            };

            matcher_actions.push(matcher_action);
        };

        Ok(matcher_actions)
    }

}

pub struct MatcherAction {
    pub id: String,
    pub payload: HashMap<String, Accessor>,
}

impl MatcherAction {

    pub fn execute(&self, event: &ProcessedEvent) -> Result<Action, MatcherError> {
        let mut action = Action{
            id: self.id.to_owned(),
            payload: HashMap::new()
        };

        for (key, accessor) in self.payload {
            let value = accessor.get(event)?;
            action.payload.insert(
                payload_key.to_owned(),
                value.to_string()
            );
        };

        Ok(action)
    }

}