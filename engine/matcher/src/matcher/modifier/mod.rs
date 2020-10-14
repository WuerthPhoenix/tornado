use crate::config::rule::Modifier;
use crate::error::MatcherError;
use tornado_common_api::Value;
use log::*;

#[derive(Debug)]
pub enum ValueModifier {
    Trim,
}

impl ValueModifier {

    pub fn build(
        _rule_name: &str,
        modifiers: &[Modifier],
    ) -> Result<Vec<ValueModifier>, MatcherError> {

        let mut value_modifiers = vec![];

        for modifier in modifiers {
            match modifier {
                Modifier::Trim {} => {
                    trace!("Add post modifier to extractor: trim");
                    value_modifiers.push(ValueModifier::Trim);
                }
            }
        };

        Ok(value_modifiers)
    }

    pub fn apply(&self, variable_name: &str, value: &mut Value) -> Result<(), MatcherError> {
        match self {
            ValueModifier::Trim => {
                if let Some(text) = value.get_text() {
                    *value = Value::Text(text.trim().to_owned());
                    Ok(())
                } else {
                    Err(MatcherError::ExtractedVariableError {
                        message: "The 'trim' modifier can be used only with values of type 'string'".to_owned(),
                        variable_name: variable_name.to_owned(),
                    })
                }
            }
        }
    }
}