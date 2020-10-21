use crate::error::MatcherError;
use tornado_common_api::Value;

#[inline]
pub fn trim(variable_name: &str, value: &mut Value) -> Result<(), MatcherError> {
    if let Some(text) = value.get_text() {
        let trimmed = text.trim();
        if trimmed.len() < text.len() {
            *value = Value::Text(trimmed.to_owned());
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

    #[test]
    fn trim_modifier_should_trim_a_string() {
        {
            let mut input = Value::Text("".to_owned());
            trim("", &mut input).unwrap();
            assert_eq!(Value::Text("".to_owned()), input);
        }

        {
            let mut input = Value::Text("not to trim".to_owned());
            trim("", &mut input).unwrap();
            assert_eq!(Value::Text("not to trim".to_owned()), input);
        }

        {
            let mut input = Value::Text(" to be trimmed  ".to_owned());
            trim("", &mut input).unwrap();
            assert_eq!(Value::Text("to be trimmed".to_owned()), input);
        }
    }

    #[test]
    fn trim_modifier_should_fail_if_value_not_a_string() {
        {
            let mut input = Value::Array(vec![]);
            assert!(trim("", &mut input).is_err());
        }

        {
            let mut input = Value::Map(HashMap::new());
            assert!(trim("", &mut input).is_err());
        }

        {
            let mut input = Value::Bool(true);
            assert!(trim("", &mut input).is_err());
        }
    }
}
