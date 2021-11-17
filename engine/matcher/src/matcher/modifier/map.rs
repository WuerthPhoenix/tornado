use crate::error::MatcherError;
use std::collections::HashMap;
use tornado_common_api::{Value};

#[inline]
pub fn map(
    variable_name: &str,
    value: &mut Value,
    mapping: &HashMap<String, String>,
    default_value: &Option<String>,
) -> Result<(), MatcherError> {
    if let Some(text) = value.get_text() {
        if let Some(mapped_value) = mapping.get(text) {
            *value = Value::Text(mapped_value.to_owned());
            Ok(())
        } else if let Some(default_text) = default_value {
            *value = Value::Text(default_text.to_owned());
            Ok(())
        } else {
            Err(MatcherError::ExtractedVariableError {
                message: format!("The 'map' modifier cannot find mapped value for [{}]", text),
                variable_name: variable_name.to_owned(),
            })
        }
    } else {
        Err(MatcherError::ExtractedVariableError {
            message: "The 'map' modifier can be used only with values of type 'string'".to_owned(),
            variable_name: variable_name.to_owned(),
        })
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;
    use tornado_common_api::Number;

    #[test]
    fn map_modifier_should_replace_a_string() {
        let mut mapping = HashMap::new();
        mapping.insert("Ok".to_owned(), "0".to_owned());
        mapping.insert("Warn".to_owned(), "1".to_owned());
        mapping.insert("Critical".to_owned(), "2".to_owned());

        {
            let default_value = None;
            let mut input = Value::Text("Ok".to_owned());
            map("", &mut input, &mapping, &default_value).unwrap();
            assert_eq!(Value::Text("0".to_owned()), input);
        }

        {
            let default_value = None;
            let mut input = Value::Text("Warn".to_owned());
            map("", &mut input, &mapping, &default_value).unwrap();
            assert_eq!(Value::Text("1".to_owned()), input);
        }

        {
            let default_value = Some("default_value".to_owned());
            let mut input = Value::Text("Critical".to_owned());
            map("", &mut input, &mapping, &default_value).unwrap();
            assert_eq!(Value::Text("2".to_owned()), input);
        }
    }

    #[test]
    fn map_modifier_should_fail_if_input_not_string() {
        let mut mapping = HashMap::new();
        mapping.insert("Ok".to_owned(), "0".to_owned());
        mapping.insert("Warn".to_owned(), "1".to_owned());
        mapping.insert("Critical".to_owned(), "2".to_owned());

        let default_value = Some("default_value".to_owned());

        {
            let mut input = Value::Number(Number::PosInt(3));
            assert!(map("", &mut input, &mapping, &default_value).is_err());
        }
    }

    #[test]
    fn map_modifier_should_fail_if_mapped_value_not_found() {
        let mut mapping = HashMap::new();
        mapping.insert("Ok".to_owned(), "0".to_owned());
        mapping.insert("Warn".to_owned(), "1".to_owned());
        mapping.insert("Critical".to_owned(), "2".to_owned());

        let default_value = None;

        {
            let mut input = Value::Text("Unknown".to_owned());
            assert!(map("", &mut input, &mapping, &default_value).is_err());
        }
    }

    #[test]
    fn map_modifier_should_fallback_to_default_if_mapped_value_not_found() {
        let mut mapping = HashMap::new();
        mapping.insert("Ok".to_owned(), "0".to_owned());
        mapping.insert("Warn".to_owned(), "1".to_owned());
        mapping.insert("Critical".to_owned(), "2".to_owned());

        let default_value = Some("default_value".to_owned());

        {
            let mut input = Value::Text("Unknown".to_owned());
            map("", &mut input, &mapping, &default_value).unwrap();
            assert_eq!(Value::Text("default_value".to_owned()), input);
        }
    }
}
