pub mod action;

use config::Rule;
use error::MatcherError;

#[derive(Default)]
pub struct Dispatcher {
    actions: Vec<action::DispatcherAction>
}

impl Dispatcher {
    pub fn new(rules: &[Rule]) -> Result<Dispatcher, MatcherError> {

        let action_builder = action::DispatcherActionBuilder::new();

        let mut dispatcher = Dispatcher{
            actions: vec![]
        };

        for rule in rules {
            //action_builder.build(&rule.name, &rule.actions)
        }

        Ok(dispatcher)
    }

}

