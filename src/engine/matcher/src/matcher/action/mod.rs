use accessor::{Accessor, AccessorBuilder};
use config::Action as ConfigAction;
use error::MatcherError;
use model::ProcessedEvent;
use std::collections::HashMap;
use tornado_common_api::Action;

#[derive(Default)]
pub struct ActionResolverBuilder {
    accessor: AccessorBuilder,
}

/// ActionResolver builder
impl ActionResolverBuilder {
    pub fn new() -> ActionResolverBuilder {
        ActionResolverBuilder { accessor: AccessorBuilder::new() }
    }

    /// Receives an array of Actions as defined in a Rule an returns an array of ActionResolver.
    /// Each ActionResolver is linked to an input Action definition and contains the logic build the final
    /// Action object ready to be sent to the executors.
    pub fn build(
        &self,
        rule_name: &str,
        actions: &[ConfigAction],
    ) -> Result<Vec<ActionResolver>, MatcherError> {
        let mut matcher_actions = vec![];

        for action in actions {
            let mut matcher_action = ActionResolver {
                rule_name: rule_name.to_owned(),
                id: action.id.to_owned(),
                payload: HashMap::new(),
            };

            for (payload_key, payload_value) in &action.payload {
                matcher_action
                    .payload
                    .insert(payload_key.to_owned(), self.accessor.build(rule_name, payload_value)?);
            }

            matcher_actions.push(matcher_action);
        }

        Ok(matcher_actions)
    }
}

/// An Action resolver creates Actions from a ProcessedEvent
pub struct ActionResolver {
    rule_name: String,
    id: String,
    payload: HashMap<String, Accessor>,
}

impl ActionResolver {
    /// Builds an Action extracting the required data from the ProcessedEvent.
    /// The outcome is a fully resolved Action ready to be processed by the executors.
    pub fn execute(&self, event: &ProcessedEvent) -> Result<Action, MatcherError> {
        let mut action = Action { id: self.id.to_owned(), payload: HashMap::new() };

        for (key, accessor) in &self.payload {
            let value = match accessor.get(event) {
                Some(value) => Ok(value),
                None => Err(MatcherError::CreateActionError {
                    action_id: self.id.to_owned(),
                    rule_name: self.rule_name.to_owned(),
                    cause: format!("Accessor [{:?}] returned empty value.", accessor),
                }),
            };
            action.payload.insert(key.to_owned(), value?.to_string());
        }

        Ok(action)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use accessor::Accessor;
    use std::collections::HashMap;
    use tornado_common_api::Event;

    #[test]
    fn should_build_a_matcher_action() {
        // Arrange
        let mut action = ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        let value = "constant value".to_owned();
        action.payload.insert("key".to_owned(), value.clone());

        let config = vec![action];

        // Act
        let actions = ActionResolverBuilder::new().build("", &config).unwrap();

        // Assert
        assert_eq!(1, actions.len());
        assert_eq!("an_action_id", &actions.get(0).unwrap().id);

        let action_payload = &actions.get(0).unwrap().payload;
        assert_eq!(1, action_payload.len());
        assert!(action_payload.contains_key("key"));
        assert_eq!(&Accessor::Constant { value }, action_payload.get("key").unwrap())
    }

    #[test]
    fn should_build_an_action() {
        // Arrange
        let mut config_action =
            ConfigAction { id: "an_action_id".to_owned(), payload: HashMap::new() };
        config_action.payload.insert("type".to_owned(), "${event.type}".to_owned());
        config_action.payload.insert("payload_body".to_owned(), "${event.payload.body}".to_owned());
        config_action
            .payload
            .insert("payload_subject".to_owned(), "${event.payload.subject}".to_owned());
        config_action.payload.insert("constant".to_owned(), "constant value".to_owned());
        config_action.payload.insert("created_ts".to_owned(), "${event.created_ts}".to_owned());
        config_action.payload.insert("var_test_1".to_owned(), "${_variables.test1}".to_owned());
        config_action.payload.insert("var_test_2".to_owned(), "${_variables.test2}".to_owned());

        let rule_name = "rule_for_test";
        let config = vec![config_action];
        let matcher_actions = ActionResolverBuilder::new().build(rule_name, &config).unwrap();
        let matcher_action = &matcher_actions[0];

        let mut event = ProcessedEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ts: 123456,
            payload: HashMap::new(),
        });

        event.event.payload.insert("body".to_owned(), "body_value".to_owned());
        event.event.payload.insert("subject".to_owned(), "subject_value".to_owned());

        event.extracted_vars.insert("rule_for_test.test1", "var_test_1_value".to_owned());
        event.extracted_vars.insert("rule_for_test.test2", "var_test_2_value".to_owned());

        // Act
        let result = matcher_action.execute(&event).unwrap();

        // Assert
        assert_eq!(&"an_action_id", &result.id);
        assert_eq!(&"event_type_value", &result.payload.get("type").unwrap());
        assert_eq!(&"body_value", &result.payload.get("payload_body").unwrap());
        assert_eq!(&"subject_value", &result.payload.get("payload_subject").unwrap());
        assert_eq!(&"constant value", &result.payload.get("constant").unwrap());
        assert_eq!(&"123456", &result.payload.get("created_ts").unwrap());
        assert_eq!(&"var_test_1_value", &result.payload.get("var_test_1").unwrap());
        assert_eq!(&"var_test_2_value", &result.payload.get("var_test_2").unwrap());
    }

}
