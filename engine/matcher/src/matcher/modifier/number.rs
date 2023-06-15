use serde_json::{json, Value};

use crate::error::MatcherError;

#[inline]
pub fn to_number(variable_name: &str, value: &mut Value) -> Result<(), MatcherError> {
    match value {
        Value::String(text) => {
            if let Ok(u_value) = text.parse::<u64>() {
                *value = json!(u_value);
                Ok(())
            } else if let Ok(i_value) = text.parse::<i64>() {
                *value = json!(i_value);
                Ok(())
            } else if let Ok(f_value) = text.parse::<f64>() {
                *value = json!(f_value);
                Ok(())
            } else {
                Err(MatcherError::ExtractedVariableError {
                    message: format!(
                        "The 'to_number' modifier cannot parse string [{}] to number",
                        text
                    ),
                    variable_name: variable_name.to_owned(),
                })
            }
        }
        Value::Number(..) => Ok(()),
        _ => Err(MatcherError::ExtractedVariableError {
            message:
                "The 'to_number' modifier can be used only with values of type 'string' or 'number'"
                    .to_owned(),
            variable_name: variable_name.to_owned(),
        }),
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::approx_constant)]

    use super::*;
    use serde_json::json;

    #[test]
    fn to_number_modifier_should_return_a_positive_number() {
        let mut input = Value::String("12".to_owned());
        to_number("", &mut input).unwrap();
        assert_eq!(json!(12), input);
    }

    #[test]
    fn to_number_modifier_should_return_a_negative_number() {
        let mut input = Value::String("-3412".to_owned());
        to_number("", &mut input).unwrap();
        assert_eq!(json!(-3412), input);
    }

    #[test]
    fn to_number_modifier_should_return_a_float() {
        let mut input = Value::String("3.14".to_owned());
        to_number("", &mut input).unwrap();
        assert_eq!(json!(3.14), input);
    }

    #[test]
    fn to_number_modifier_should_return_a_error() {
        let mut input = Value::String("something".to_owned());
        assert!(to_number("", &mut input).is_err());
    }
}
