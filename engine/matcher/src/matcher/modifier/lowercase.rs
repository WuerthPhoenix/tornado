use crate::error::MatcherError;
use tornado_common_api::Value;

#[inline]
pub fn lowercase(variable_name: &str, value: &mut Value) -> Result<(), MatcherError> {
    if let Some(text) = value.get_text() {
        *value = Value::Text(text.to_lowercase());
        Ok(())
    } else {
        Err(MatcherError::ExtractedVariableError {
            message: "The 'lowercase' modifier can be used only with values of type 'string'"
                .to_owned(),
            variable_name: variable_name.to_owned(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn lowercase_modifier_should_lowercase_a_string() {
        {
            let mut input = Value::Text("".to_owned());
            lowercase("", &mut input).unwrap();
            assert_eq!(Value::Text("".to_owned()), input);
        }

        {
            let mut input = Value::Text("not to lowecase".to_owned());
            lowercase("", &mut input).unwrap();
            assert_eq!(Value::Text("not to lowecase".to_owned()), input);
        }

        {
            let mut input = Value::Text(" To BE LOwerCASEd  ".to_owned());
            lowercase("", &mut input).unwrap();
            assert_eq!(Value::Text(" to be lowercased  ".to_owned()), input);
        }
    }

    #[test]
    fn lowercase_modifier_should_fail_if_value_not_a_string() {
        {
            let mut input = Value::Array(vec![]);
            assert!(lowercase("", &mut input).is_err());
        }

        {
            let mut input = Value::Map(HashMap::new());
            assert!(lowercase("", &mut input).is_err());
        }

        {
            let mut input = Value::Bool(true);
            assert!(lowercase("", &mut input).is_err());
        }
    }
}
