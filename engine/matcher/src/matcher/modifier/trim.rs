use serde_json::Value;
use tornado_common_api::ValueExt;

use crate::error::MatcherError;

#[inline]
pub fn trim(variable_name: &str, value: &mut Value) -> Result<(), MatcherError> {
    if let Some(text) = value.get_text() {
        let trimmed = text.trim();
        if trimmed.len() < text.len() {
            *value = Value::String(trimmed.to_owned());
        }
        Ok(())
    } else {
        Err(MatcherError::ExtractedVariableError {
            message: "The 'trim' modifier can be used only with values of type 'string'".to_owned(),
            variable_name: variable_name.to_owned(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;
    use serde_json::Map;

    #[test]
    fn trim_modifier_should_trim_a_string() {
        {
            let mut input = Value::String("".to_owned());
            trim("", &mut input).unwrap();
            assert_eq!(Value::String("".to_owned()), input);
        }

        {
            let mut input = Value::String("not to trim".to_owned());
            trim("", &mut input).unwrap();
            assert_eq!(Value::String("not to trim".to_owned()), input);
        }

        {
            let mut input = Value::String(" to be trimmed  ".to_owned());
            trim("", &mut input).unwrap();
            assert_eq!(Value::String("to be trimmed".to_owned()), input);
        }
    }

    #[test]
    fn trim_modifier_should_fail_if_value_not_a_string() {
        {
            let mut input = Value::Array(vec![]);
            assert!(trim("", &mut input).is_err());
        }

        {
            let mut input = Value::Object(Map::new());
            assert!(trim("", &mut input).is_err());
        }

        {
            let mut input = Value::Bool(true);
            assert!(trim("", &mut input).is_err());
        }
    }
}
