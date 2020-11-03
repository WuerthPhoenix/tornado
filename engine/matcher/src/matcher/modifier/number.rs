use crate::error::MatcherError;
use tornado_common_api::{Number, Value};

#[inline]
pub fn to_number(variable_name: &str, value: &mut Value) -> Result<(), MatcherError> {
    match value {
        Value::Text(text) => {
            //*value = Value::Text(trimmed.to_owned());
            if let Ok(u_value) = text.parse::<u64>() {
                *value = Value::Number(Number::PosInt(u_value));
                Ok(())
            } else if let Ok(i_value) = text.parse::<i64>() {
                *value = Value::Number(Number::NegInt(i_value));
                Ok(())
            } else if let Ok(f_value) = text.parse::<f64>() {
                *value = Value::Number(Number::Float(f_value));
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
