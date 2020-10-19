use crate::config::rule::Modifier;
use crate::error::MatcherError;
use log::*;
use tornado_common_api::Value;

pub mod trim;

#[derive(Debug, PartialEq)]
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
        }

        Ok(value_modifiers)
    }

    pub fn apply(&self, variable_name: &str, value: &mut Value) -> Result<(), MatcherError> {
        match self {
            ValueModifier::Trim => trim::trim(variable_name, value),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn should_build_empty_value_modifiers() {
        // Arrange
        let modifiers = vec![];

        // Act
        let value_modifiers = ValueModifier::build("", &modifiers).unwrap();

        // Assert
        assert!(value_modifiers.is_empty());
    }

    #[test]
    fn should_build_trim_value_modifiers() {
        // Arrange
        let modifiers = vec![Modifier::Trim {}, Modifier::Trim {}];
        let expected_value_modifiers = vec![ValueModifier::Trim, ValueModifier::Trim];

        // Act
        let value_modifiers = ValueModifier::build("", &modifiers).unwrap();

        // Assert
        assert_eq!(2, value_modifiers.len());
        assert_eq!(expected_value_modifiers, value_modifiers);
    }

    #[test]
    fn trim_modifier_should_trim_a_string() {
        // Arrange
        let value_modifier = ValueModifier::Trim;

        // Act & Assert
        {
            let mut input = Value::Text("".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("".to_owned()), input);
        }

        {
            let mut input = Value::Text("not to trim".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("not to trim".to_owned()), input);
        }

        {
            let mut input = Value::Text(" to be trimmed  ".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("to be trimmed".to_owned()), input);
        }
    }
}
