use crate::accessor::{Accessor, AccessorBuilder};
use crate::config::rule::Modifier;
use crate::error::MatcherError;
use crate::model::InternalEvent;
use crate::regex::RegexWrapper;
use log::*;
use serde_json::Value;
use std::collections::HashMap;

pub mod lowercase;
pub mod map;
pub mod number;
pub mod replace;
pub mod trim;

#[derive(Debug, PartialEq)]
pub enum ValueModifier {
    Lowercase,
    Map { mapping: HashMap<String, String>, default_value: Option<String> },
    ReplaceAll { find: String, replace: Accessor },
    ReplaceAllRegex { find_regex: RegexWrapper, replace: Accessor },
    ToNumber,
    Trim,
}

impl ValueModifier {
    pub fn build(
        rule_name: &str,
        accessor_builder: &AccessorBuilder,
        modifiers: &[Modifier],
    ) -> Result<Vec<ValueModifier>, MatcherError> {
        let mut value_modifiers = vec![];

        for modifier in modifiers {
            match modifier {
                Modifier::Lowercase {} => {
                    trace!("Add post modifier to extractor: lowercase");
                    value_modifiers.push(ValueModifier::Lowercase);
                }
                Modifier::Map { mapping, default_value } => {
                    trace!(
                        "Add post modifier to extractor: Map: {:?}; default_value: {:?}",
                        mapping,
                        default_value
                    );
                    value_modifiers.push(ValueModifier::Map {
                        mapping: mapping.clone(),
                        default_value: default_value.clone(),
                    });
                }
                Modifier::ReplaceAll { find, replace, is_regex } => {
                    trace!("Add post modifier to extractor: replace. Is it regex? {}", is_regex);
                    if *is_regex {
                        value_modifiers.push(ValueModifier::ReplaceAllRegex {
                            find_regex: RegexWrapper::new(find)?,
                            replace: accessor_builder.build(rule_name, replace)?,
                        });
                    } else {
                        value_modifiers.push(ValueModifier::ReplaceAll {
                            find: find.clone(),
                            replace: accessor_builder.build(rule_name, replace)?,
                        });
                    }
                }
                Modifier::ToNumber {} => {
                    trace!("Add post modifier to extractor: ToNumber");
                    value_modifiers.push(ValueModifier::ToNumber);
                }
                Modifier::Trim {} => {
                    trace!("Add post modifier to extractor: trim");
                    value_modifiers.push(ValueModifier::Trim);
                }
            }
        }

        Ok(value_modifiers)
    }

    pub fn apply(
        &self,
        variable_name: &str,
        value: &mut Value,
        event: &InternalEvent,
        extracted_vars: Option<&Value>,
    ) -> Result<(), MatcherError> {
        match self {
            ValueModifier::Lowercase => lowercase::lowercase(variable_name, value),
            ValueModifier::Map { mapping, default_value } => {
                map::map(variable_name, value, mapping, default_value)
            }
            ValueModifier::ReplaceAll { find, replace } => {
                replace::replace_all(variable_name, value, find, replace, event, extracted_vars)
            }
            ValueModifier::ReplaceAllRegex { find_regex, replace } => {
                replace::replace_all_with_regex(
                    variable_name,
                    value,
                    find_regex,
                    replace,
                    event,
                    extracted_vars,
                )
            }
            ValueModifier::ToNumber => number::to_number(variable_name, value),
            ValueModifier::Trim => trim::trim(variable_name, value),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::*;
    use tornado_common_api::{Event, Number};

    #[test]
    fn should_build_empty_value_modifiers() {
        // Arrange
        let modifiers = vec![];

        // Act
        let value_modifiers =
            ValueModifier::build("", &AccessorBuilder::new(), &modifiers).unwrap();

        // Assert
        assert!(value_modifiers.is_empty());
    }

    #[test]
    fn should_build_trim_value_modifiers() {
        // Arrange
        let modifiers = vec![Modifier::Trim {}, Modifier::Trim {}];
        let expected_value_modifiers = vec![ValueModifier::Trim, ValueModifier::Trim];

        // Act
        let value_modifiers =
            ValueModifier::build("", &AccessorBuilder::new(), &modifiers).unwrap();

        // Assert
        assert_eq!(2, value_modifiers.len());
        assert_eq!(expected_value_modifiers, value_modifiers);
    }

    #[test]
    fn should_build_lowercase_value_modifiers() {
        // Arrange
        let modifiers = vec![Modifier::Lowercase {}, Modifier::Trim {}];
        let expected_value_modifiers = vec![ValueModifier::Lowercase, ValueModifier::Trim];

        // Act
        let value_modifiers =
            ValueModifier::build("", &AccessorBuilder::new(), &modifiers).unwrap();

        // Assert
        assert_eq!(2, value_modifiers.len());
        assert_eq!(expected_value_modifiers, value_modifiers);
    }

    #[test]
    fn should_build_to_number_value_modifiers() {
        // Arrange
        let modifiers = vec![Modifier::ToNumber {}];
        let expected_value_modifiers = vec![ValueModifier::ToNumber];

        // Act
        let value_modifiers =
            ValueModifier::build("", &AccessorBuilder::new(), &modifiers).unwrap();

        // Assert
        assert_eq!(1, value_modifiers.len());
        assert_eq!(expected_value_modifiers, value_modifiers);
    }

    #[test]
    fn should_build_a_replace_all_value_modifiers() {
        // Arrange
        let modifiers = vec![Modifier::ReplaceAll {
            is_regex: false,
            find: "some".to_owned(),
            replace: "some other".to_owned(),
        }];
        let expected_value_modifiers = vec![ValueModifier::ReplaceAll {
            find: "some".to_owned(),
            replace: AccessorBuilder::new().build("", "some other").unwrap(),
        }];

        // Act
        let value_modifiers =
            ValueModifier::build("", &AccessorBuilder::new(), &modifiers).unwrap();

        // Assert
        assert_eq!(1, value_modifiers.len());
        assert_eq!(expected_value_modifiers, value_modifiers);
    }

    #[test]
    fn should_build_a_map_modifiers() {
        // Arrange
        let modifiers = vec![Modifier::Map {
            default_value: Some("Keith Richards".to_owned()),
            mapping: hashmap!(
                "0".to_owned() => "David Gilmour".to_owned(),
            ),
        }];

        let expected_value_modifiers = vec![ValueModifier::Map {
            default_value: Some("Keith Richards".to_owned()),
            mapping: hashmap!(
                "0".to_owned() => "David Gilmour".to_owned(),
            ),
        }];

        // Act
        let value_modifiers =
            ValueModifier::build("", &AccessorBuilder::new(), &modifiers).unwrap();

        // Assert
        assert_eq!(1, value_modifiers.len());
        assert_eq!(expected_value_modifiers, value_modifiers);
    }

    #[test]
    fn should_build_a_replace_all_with_regex_value_modifiers() {
        // Arrange
        let modifiers = vec![Modifier::ReplaceAll {
            is_regex: true,
            find: "./*".to_owned(),
            replace: "some other".to_owned(),
        }];
        let expected_value_modifiers = vec![ValueModifier::ReplaceAllRegex {
            find_regex: RegexWrapper::new("./*").unwrap(),
            replace: AccessorBuilder::new().build("", "some other").unwrap(),
        }];

        // Act
        let value_modifiers =
            ValueModifier::build("", &AccessorBuilder::new(), &modifiers).unwrap();

        // Assert
        assert_eq!(1, value_modifiers.len());
        assert_eq!(expected_value_modifiers, value_modifiers);
    }

    #[test]
    fn build_should_fail_if_replace_all_has_invalid_regex() {
        // Arrange
        let modifiers = vec![Modifier::ReplaceAll {
            is_regex: true,
            find: "[".to_owned(),
            replace: "some other".to_owned(),
        }];

        // Act & Assert
        assert!(ValueModifier::build("", &AccessorBuilder::new(), &modifiers).is_err());
    }

    #[test]
    fn trim_modifier_should_trim_a_string() {
        // Arrange
        let event = InternalEvent::new(Event::new(""));
        let variables = None;
        let value_modifier = ValueModifier::Trim;

        // Act & Assert
        {
            let mut input = Value::String("".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::String("".to_owned()), input);
        }

        {
            let mut input = Value::String("not to trim".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::String("not to trim".to_owned()), input);
        }

        {
            let mut input = Value::String(" to be trimmed  ".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::String("to be trimmed".to_owned()), input);
        }
    }

    #[test]
    fn lowercase_modifier_should_lowercase_a_string() {
        // Arrange
        let event = InternalEvent::new(Event::new(""));
        let variables = None;
        let value_modifier = ValueModifier::Lowercase;

        // Act & Assert
        {
            let mut input = Value::String("".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::String("".to_owned()), input);
        }

        {
            let mut input = Value::String("ok".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::String("ok".to_owned()), input);
        }

        {
            let mut input = Value::String("OK".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::String("ok".to_owned()), input);
        }
    }

    #[test]
    fn replace_modifier_should_replace_a_string() {
        // Arrange
        let event = InternalEvent::new(Event::new(""));
        let variables = None;
        let value_modifier = ValueModifier::ReplaceAll {
            find: "Hello".to_owned(),
            replace: AccessorBuilder::new().build("", "World").unwrap(),
        };

        // Act & Assert
        {
            let mut input = Value::String("Hello World".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::String("World World".to_owned()), input);
        }
    }

    #[test]
    fn to_number_modifier_should_return_a_number() {
        // Arrange
        let event = InternalEvent::new(Event::new(""));
        let variables = None;
        let value_modifier = ValueModifier::ToNumber;

        // Act & Assert
        {
            let mut input = Value::String("12".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::Number(Number::PosInt(12)), input);
        }

        {
            let mut input = Value::String("-3412".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::Number(Number::NegInt(-3412)), input);
        }

        {
            let mut input = Value::String("3.14".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::Number(Number::Float(3.14)), input);
        }

        {
            let mut input = Value::String("something".to_owned());
            assert!(value_modifier.apply("", &mut input, &event, variables).is_err());
        }
    }

    #[test]
    fn replace_with_regex_modifier_should_replace_a_string() {
        // Arrange
        let event = InternalEvent::new(Event::new(""));
        let variables = None;
        let value_modifier = ValueModifier::ReplaceAllRegex {
            find_regex: RegexWrapper::new("[0-9]+").unwrap(),
            replace: AccessorBuilder::new().build("", "number").unwrap(),
        };

        // Act & Assert
        {
            let mut input = Value::String("Hello World 123 4!".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::String("Hello World number number!".to_owned()), input);
        }
    }

    #[test]
    fn map_modifier_should_replace_a_string() {
        // Arrange
        let event = InternalEvent::new(Event::new(""));
        let variables = None;
        let value_modifier = ValueModifier::Map {
            default_value: Some("Keith Richards".to_owned()),
            mapping: hashmap!(
                "0".to_owned() => "David Gilmour".to_owned(),
            ),
        };

        // Act & Assert
        {
            let mut input = Value::String("0".to_owned());
            value_modifier.apply("", &mut input, &event, variables).unwrap();
            assert_eq!(Value::String("David Gilmour".to_owned()), input);
        }
    }
}
