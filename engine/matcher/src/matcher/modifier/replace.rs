use crate::error::MatcherError;
use regex::Regex;
use tornado_common_api::Value;

#[inline]
pub fn replace_all(
    variable_name: &str,
    value: &mut Value,
    find: &str,
    replace: &str,
) -> Result<(), MatcherError> {
    if let Some(text) = value.get_text() {
        if text.contains(find) {
            *value = Value::Text(text.replace(find, replace));
        }
        Ok(())
    } else {
        Err(MatcherError::ExtractedVariableError {
            message: "The 'replace' modifier can be used only with values of type 'string'"
                .to_owned(),
            variable_name: variable_name.to_owned(),
        })
    }
}

#[inline]
pub fn replace_all_with_regex(
    variable_name: &str,
    value: &mut Value,
    find_regex: &Regex,
    replace: &str,
) -> Result<(), MatcherError> {
    if let Some(text) = value.get_text() {
        let result = find_regex.replace_all(text, replace);
        *value = Value::Text(result.into_owned());
        Ok(())
    } else {
        Err(MatcherError::ExtractedVariableError {
            message: "The 'replace' modifier can be used only with values of type 'string'"
                .to_owned(),
            variable_name: variable_name.to_owned(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::regex::RegexWrapper;
    use std::collections::HashMap;

    #[test]
    fn replace_all_modifier_should_replace_a_string() {
        let find_text = "text";
        let replace_text = "new_text";

        {
            let mut input = Value::Text("".to_owned());
            replace_all("", &mut input, find_text, replace_text).unwrap();
            assert_eq!(Value::Text("".to_owned()), input);
        }

        {
            let mut input = Value::Text("not to replace".to_owned());
            replace_all("", &mut input, find_text, replace_text).unwrap();
            assert_eq!(Value::Text("not to replace".to_owned()), input);
        }

        {
            let mut input = Value::Text("to replace text".to_owned());
            replace_all("", &mut input, find_text, replace_text).unwrap();
            assert_eq!(Value::Text("to replace new_text".to_owned()), input);
        }

        {
            let mut input = Value::Text("to replace text and text".to_owned());
            replace_all("", &mut input, find_text, replace_text).unwrap();
            assert_eq!(Value::Text("to replace new_text and new_text".to_owned()), input);
        }
    }

    #[test]
    fn replace_all_modifier_should_be_case_sensitive() {
        let find_text = "TexT";
        let replace_text = "new_TexT";

        {
            let mut input = Value::Text("text".to_owned());
            replace_all("", &mut input, find_text, replace_text).unwrap();
            assert_eq!(Value::Text("text".to_owned()), input);
        }

        {
            let mut input = Value::Text("TexT".to_owned());
            replace_all("", &mut input, find_text, replace_text).unwrap();
            assert_eq!(Value::Text("new_TexT".to_owned()), input);
        }
    }

    #[test]
    fn replace_all_modifier_should_fail_if_value_not_a_string() {
        let find_text = "text";
        let replace_text = "new_text";

        {
            let mut input = Value::Array(vec![]);
            assert!(replace_all("", &mut input, find_text, replace_text).is_err());
        }

        {
            let mut input = Value::Map(HashMap::new());
            assert!(replace_all("", &mut input, find_text, replace_text).is_err());
        }

        {
            let mut input = Value::Bool(true);
            assert!(replace_all("", &mut input, find_text, replace_text).is_err());
        }
    }

    #[test]
    fn replace_all_with_regex_modifier_should_replace_a_string() {
        let find_regex = RegexWrapper::new("[0-9]+").unwrap();
        let replace_text = "replaced";

        {
            let mut input = Value::Text("".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, replace_text).unwrap();
            assert_eq!(Value::Text("".to_owned()), input);
        }

        {
            let mut input = Value::Text("not to replace".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, replace_text).unwrap();
            assert_eq!(Value::Text("not to replace".to_owned()), input);
        }

        {
            let mut input = Value::Text("to replace 12 and 3".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, replace_text).unwrap();
            assert_eq!(Value::Text("to replace replaced and replaced".to_owned()), input);
        }
    }

    #[test]
    fn replace_all_with_regex_modifier_should_allow_named_groups() {
        let find_regex = RegexWrapper::new(r"(?P<last>[^,\s]+),\s+(?P<first>\S+)").unwrap();
        let replace_text = "$first $last";

        {
            let mut input = Value::Text("Springsteen, Bruce".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, replace_text).unwrap();
            assert_eq!(Value::Text("Bruce Springsteen".to_owned()), input);
        }
    }

    #[test]
    fn replace_all_with_regex_modifier_should_allow_positional_groups() {
        let find_regex = RegexWrapper::new(r"(?P<last>[^,\s]+),\s+(?P<first>\S+)").unwrap();
        let replace_text = "$2 $1";

        {
            let mut input = Value::Text("Deacon, John".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, replace_text).unwrap();
            assert_eq!(Value::Text("John Deacon".to_owned()), input);
        }
    }
}
