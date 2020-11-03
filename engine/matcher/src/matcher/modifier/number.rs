use crate::error::MatcherError;
use tornado_common_api::Value;

#[inline]
pub fn to_number(variable_name: &str, value: &mut Value) -> Result<(), MatcherError> {
    match value {
        Value::Text(text) => {
            //*value = Value::Text(trimmed.to_owned());
            Ok(())
        }
        Value::Number(..) => Ok(()),
        _ => Err(MatcherError::ExtractedVariableError {
            message: "The 'to_number' modifier can be used only with values of type 'string' or 'number'".to_owned(),
            variable_name: variable_name.to_owned(),
        })
    }
}