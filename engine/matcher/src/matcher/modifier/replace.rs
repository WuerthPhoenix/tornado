use crate::accessor::Accessor;
use crate::error::MatcherError;
use crate::model::InternalEvent;
use regex::Regex;
use serde_json::Value;
use tornado_common_api::ValueExt;

#[inline]
pub fn replace_all(
    variable_name: &str,
    value: &mut Value,
    find: &str,
    replace: &Accessor,
    event: &InternalEvent,
    extracted_vars: Option<&Value>,
) -> Result<(), MatcherError> {
    if let Some(text) = value.get_text() {
        if text.contains(find) {
            if let Some(replace_get) = replace.get(event, extracted_vars) {
                if let Some(replace_value) = replace_get.get_text() {
                    *value = Value::String(text.replace(find, replace_value));
                    return Ok(());
                }
            }
        } else {
            return Ok(());
        }
    };
    Err(MatcherError::ExtractedVariableError {
        message: "The 'replace' modifier can be used only with values of type 'string'".to_owned(),
        variable_name: variable_name.to_owned(),
    })
}

#[inline]
pub fn replace_all_with_regex(
    variable_name: &str,
    value: &mut Value,
    find_regex: &Regex,
    replace: &Accessor,
    event: &InternalEvent,
    extracted_vars: Option<&Value>,
) -> Result<(), MatcherError> {
    if let Some(text) = value.get_text() {
        if let Some(replace_get) = replace.get(event, extracted_vars) {
            if let Some(replace_value) = replace_get.get_text() {
                let result = find_regex.replace_all(text, replace_value);
                *value = Value::String(result.into_owned());
                return Ok(());
            }
        }
    };

    Err(MatcherError::ExtractedVariableError {
        message: "The 'replace' modifier can be used only with values of type 'string'".to_owned(),
        variable_name: variable_name.to_owned(),
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::accessor::AccessorBuilder;
    use crate::regex::RegexWrapper;
    use maplit::*;
    use tornado_common_api::{Event, Map};

    #[test]
    fn replace_all_modifier_should_replace_a_string() {
        let find_text = "text";
        let replace_text = AccessorBuilder::new().build("", "new_text").unwrap();
        let event = InternalEvent::new(Event::new(""));
        let variables = None;

        {
            let mut input = Value::String("".to_owned());
            replace_all("", &mut input, find_text, &replace_text, &event, variables).unwrap();
            assert_eq!(Value::String("".to_owned()), input);
        }

        {
            let mut input = Value::String("not to replace".to_owned());
            replace_all("", &mut input, find_text, &replace_text, &event, variables).unwrap();
            assert_eq!(Value::String("not to replace".to_owned()), input);
        }

        {
            let mut input = Value::String("to replace text".to_owned());
            replace_all("", &mut input, find_text, &replace_text, &event, variables).unwrap();
            assert_eq!(Value::String("to replace new_text".to_owned()), input);
        }

        {
            let mut input = Value::String("to replace text and text".to_owned());
            replace_all("", &mut input, find_text, &replace_text, &event, variables).unwrap();
            assert_eq!(Value::String("to replace new_text and new_text".to_owned()), input);
        }
    }

    #[test]
    fn replace_all_modifier_should_extract_data_from_event() {
        let find_text = "text";
        let replace_text = AccessorBuilder::new().build("", "${event.payload.key_1}").unwrap();
        println!("{:#?}", replace_text);

        let event = InternalEvent::new(Event::new_with_payload(
            "my_type",
            hashmap!(
                "key_1".to_owned() => Value::String("value_1_from_payload".to_owned()),
            ),
        ));
        let variables = None;

        {
            let mut input = Value::String("this is text".to_owned());
            replace_all("", &mut input, find_text, &replace_text, &event, variables).unwrap();
            assert_eq!(Value::String("this is value_1_from_payload".to_owned()), input);
        }
    }

    // To be fixed in TOR-289
    // #[test]
    // fn replace_all_modifier_should_interpolate_extract_data_from_event() {
    //     let find_text = "text";
    //     let replace_text =
    //         AccessorBuilder::new().build("", "new_text and ${event.payload.key_1}").unwrap();
    //     println!("{:#?}", replace_text);
    //
    //     let event = InternalEvent::new(Event::new_with_payload(
    //         "my_type",
    //         hashmap!(
    //             "key_1".to_owned() => Value::String("value_1_from_payload".to_owned()),
    //         ),
    //     ));
    //     let variables = None;
    //
    //     {
    //         let mut input = Value::String("this is text".to_owned());
    //         replace_all("", &mut input, find_text, &replace_text, &event, variables).unwrap();
    //         assert_eq!(Value::String("this is new_text and value_1_from_payload".to_owned()), input);
    //     }
    // }

    #[test]
    fn replace_all_modifier_should_be_case_sensitive() {
        let find_text = "TexT";
        let replace_text = AccessorBuilder::new().build("", "new_TexT").unwrap();
        let event = InternalEvent::new(Event::new(""));
        let variables = None;

        {
            let mut input = Value::String("text".to_owned());
            replace_all("", &mut input, find_text, &replace_text, &event, variables).unwrap();
            assert_eq!(Value::String("text".to_owned()), input);
        }

        {
            let mut input = Value::String("TexT".to_owned());
            replace_all("", &mut input, find_text, &replace_text, &event, variables).unwrap();
            assert_eq!(Value::String("new_TexT".to_owned()), input);
        }
    }

    #[test]
    fn replace_all_modifier_should_fail_if_value_not_a_string() {
        let find_text = "text";
        let replace_text = AccessorBuilder::new().build("", "new_text").unwrap();
        let event = InternalEvent::new(Event::new(""));
        let variables = None;

        {
            let mut input = Value::Array(vec![]);
            assert!(
                replace_all("", &mut input, find_text, &replace_text, &event, variables).is_err()
            );
        }

        {
            let mut input = Value::Object(Map::new());
            assert!(
                replace_all("", &mut input, find_text, &replace_text, &event, variables).is_err()
            );
        }

        {
            let mut input = Value::Bool(true);
            assert!(
                replace_all("", &mut input, find_text, &replace_text, &event, variables).is_err()
            );
        }
    }

    #[test]
    fn replace_all_with_regex_modifier_should_replace_a_string() {
        let find_regex = RegexWrapper::new("[0-9]+").unwrap();
        let replace_text = AccessorBuilder::new().build("", "replaced").unwrap();
        let event = InternalEvent::new(Event::new(""));
        let variables = None;

        {
            let mut input = Value::String("".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, &replace_text, &event, variables)
                .unwrap();
            assert_eq!(Value::String("".to_owned()), input);
        }

        {
            let mut input = Value::String("not to replace".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, &replace_text, &event, variables)
                .unwrap();
            assert_eq!(Value::String("not to replace".to_owned()), input);
        }

        {
            let mut input = Value::String("to replace 12 and 3".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, &replace_text, &event, variables)
                .unwrap();
            assert_eq!(Value::String("to replace replaced and replaced".to_owned()), input);
        }
    }

    #[test]
    fn replace_all_with_regex_modifier_should_allow_named_groups() {
        let find_regex = RegexWrapper::new(r"(?P<last>[^,\s]+),\s+(?P<first>\S+)").unwrap();
        let replace_text = AccessorBuilder::new().build("", "$first $last").unwrap();
        let event = InternalEvent::new(Event::new(""));
        let variables = None;

        {
            let mut input = Value::String("Springsteen, Bruce".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, &replace_text, &event, variables)
                .unwrap();
            assert_eq!(Value::String("Bruce Springsteen".to_owned()), input);
        }
    }

    #[test]
    fn replace_all_with_regex_modifier_should_allow_positional_groups() {
        let find_regex = RegexWrapper::new(r"(?P<last>[^,\s]+),\s+(?P<first>\S+)").unwrap();
        let replace_text = AccessorBuilder::new().build("", "$2 $1").unwrap();
        let event = InternalEvent::new(Event::new(""));
        let variables = None;

        {
            let mut input = Value::String("Deacon, John".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, &replace_text, &event, variables)
                .unwrap();
            assert_eq!(Value::String("John Deacon".to_owned()), input);
        }
    }

    #[test]
    fn replace_all_with_regex_modifier_should_extract_data_from_event() {
        let find_regex = RegexWrapper::new(r"(?P<last>[^,\s]+),\s+(?P<first>\S+./*)").unwrap();
        let replace_text = AccessorBuilder::new().build("", "${event.payload.role}").unwrap();
        println!("{:#?}", replace_text);

        let event = InternalEvent::new(Event::new_with_payload(
            "my_type",
            hashmap!(
                "role".to_owned() => Value::String("$first $last: Great Bass Player".to_owned()),
            ),
        ));
        let variables = None;

        {
            let mut input = Value::String("Deacon, John".to_owned());
            replace_all_with_regex("", &mut input, &find_regex, &replace_text, &event, variables)
                .unwrap();
            assert_eq!(Value::String("John Deacon: Great Bass Player".to_owned()), input);
        }
    }

    // To be fixed in TOR-289
    // #[test]
    // fn replace_all_with_modifier_should_interpolate_extract_data_from_event_with_positional_groups()
    // {
    //     let find_regex = RegexWrapper::new(r"(?P<last>[^,\s]+),\s+(?P<first>\S+./*)").unwrap();
    //     let replace_text =
    //         AccessorBuilder::new().build("", "$2 $1: ${event.payload.role}").unwrap();
    //     println!("{:#?}", replace_text);
    //
    //     let event = InternalEvent::new(Event::new_with_payload(
    //         "my_type",
    //         hashmap!(
    //             "role".to_owned() => Value::String("Great Bass Player".to_owned()),
    //         ),
    //     ));
    //     let variables = None;
    //
    //     {
    //         let mut input = Value::String("Deacon, John".to_owned());
    //         replace_all_with_regex("", &mut input, &find_regex, &replace_text, &event, variables)
    //             .unwrap();
    //         assert_eq!(Value::String("John Deacon: Great Bass Player".to_owned()), input);
    //     }
    // }

    // To be fixed in TOR-289
    // #[test]
    // fn replace_all_with_modifier_should_interpolate_extract_data_from_event_with_named_groups() {
    //     let find_regex = RegexWrapper::new(r"(?P<last>[^,\s]+),\s+(?P<first>\S+./*)").unwrap();
    //     let replace_text =
    //         AccessorBuilder::new().build("", "$first $last: ${event.payload.role}").unwrap();
    //     println!("{:#?}", replace_text);
    //
    //     let event = InternalEvent::new(Event::new_with_payload(
    //         "my_type",
    //         hashmap!(
    //             "role".to_owned() => Value::String("Great Bass Player".to_owned()),
    //         ),
    //     ));
    //     let variables = None;
    //
    //     {
    //         let mut input = Value::String("Deacon, John".to_owned());
    //         replace_all_with_regex("", &mut input, &find_regex, &replace_text, &event, variables)
    //             .unwrap();
    //         assert_eq!(Value::String("John Deacon: Great Bass Player".to_owned()), input);
    //     }
    // }
}
