use crate::config::rule::Modifier;
use crate::error::MatcherError;
use log::*;
use regex::Regex;
use std::ops::Deref;
use tornado_common_api::Value;

pub mod lowercase;
pub mod number;
pub mod replace;
pub mod trim;

#[derive(Debug, PartialEq)]
pub enum ValueModifier {
    Lowercase,
    ReplaceAll { find: String, replace: String },
    ReplaceAllRegex { find_regex: RegexWrapper, replace: String },
    ToNumber,
    Trim,
}

#[derive(Debug)]
pub struct RegexWrapper {
    regex_string: String,
    regex: Regex,
}

impl RegexWrapper {
    pub fn new<S: Into<String>>(regex_string: S) -> Result<Self, MatcherError> {
        let regex_string = regex_string.into();
        let regex =
            Regex::new(&regex_string).map_err(|e| MatcherError::ExtractorBuildFailError {
                message: format!("Cannot parse regex [{}]", regex_string),
                cause: e.to_string(),
            })?;
        Ok(Self { regex, regex_string })
    }

    pub fn regex(&self) -> &Regex {
        &self.regex
    }
}

impl Deref for RegexWrapper {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        self.regex()
    }
}

impl PartialEq for RegexWrapper {
    fn eq(&self, other: &Self) -> bool {
        other.regex_string.eq(&self.regex_string)
    }
}

impl ValueModifier {
    pub fn build(
        _rule_name: &str,
        modifiers: &[Modifier],
    ) -> Result<Vec<ValueModifier>, MatcherError> {
        let mut value_modifiers = vec![];

        for modifier in modifiers {
            match modifier {
                Modifier::Lowercase {} => {
                    trace!("Add post modifier to extractor: lowercase");
                    value_modifiers.push(ValueModifier::Lowercase);
                }
                Modifier::ReplaceAll { find, replace, is_regex } => {
                    trace!("Add post modifier to extractor: replace. Is it regex? {}", is_regex);
                    if *is_regex {
                        value_modifiers.push(ValueModifier::ReplaceAllRegex {
                            find_regex: RegexWrapper::new(find)?,
                            replace: replace.clone(),
                        });
                    } else {
                        value_modifiers.push(ValueModifier::ReplaceAll {
                            find: find.clone(),
                            replace: replace.clone(),
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

    pub fn apply(&self, variable_name: &str, value: &mut Value) -> Result<(), MatcherError> {
        match self {
            ValueModifier::Lowercase => lowercase::lowercase(variable_name, value),
            ValueModifier::ReplaceAll { find, replace } => {
                replace::replace_all(variable_name, value, find, replace)
            }
            ValueModifier::ReplaceAllRegex { find_regex, replace } => {
                replace::replace_all_with_regex(variable_name, value, find_regex, replace)
            }
            ValueModifier::ToNumber => number::to_number(variable_name, value),
            ValueModifier::Trim => trim::trim(variable_name, value),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tornado_common_api::Number;

    #[test]
    fn should_build_empty_value_modifiers() {
        // Arrange
        let modifiers = vec![];

        // Act
        let value_modifiers = ValueModifier::build("", &modifiers).unwrap();

        // Assert
        assert!(value_modifiers.is_empty());
    }

    #[test]
    fn should_build_trim_value_modifiers() {
        // Arrange
        let modifiers = vec![Modifier::Trim {}, Modifier::Trim {}];
        let expected_value_modifiers = vec![ValueModifier::Trim, ValueModifier::Trim];

        // Act
        let value_modifiers = ValueModifier::build("", &modifiers).unwrap();

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
        let value_modifiers = ValueModifier::build("", &modifiers).unwrap();

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
        let value_modifiers = ValueModifier::build("", &modifiers).unwrap();

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
            replace: "some other".to_owned(),
        }];

        // Act
        let value_modifiers = ValueModifier::build("", &modifiers).unwrap();

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
            replace: "some other".to_owned(),
        }];

        // Act
        let value_modifiers = ValueModifier::build("", &modifiers).unwrap();

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
        assert!(ValueModifier::build("", &modifiers).is_err());
    }

    #[test]
    fn trim_modifier_should_trim_a_string() {
        // Arrange
        let value_modifier = ValueModifier::Trim;

        // Act & Assert
        {
            let mut input = Value::Text("".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("".to_owned()), input);
        }

        {
            let mut input = Value::Text("not to trim".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("not to trim".to_owned()), input);
        }

        {
            let mut input = Value::Text(" to be trimmed  ".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("to be trimmed".to_owned()), input);
        }
    }

    #[test]
    fn lowercase_modifier_should_lowercase_a_string() {
        // Arrange
        let value_modifier = ValueModifier::Lowercase;

        // Act & Assert
        {
            let mut input = Value::Text("".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("".to_owned()), input);
        }

        {
            let mut input = Value::Text("ok".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("ok".to_owned()), input);
        }

        {
            let mut input = Value::Text("OK".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("ok".to_owned()), input);
        }
    }

    #[test]
    fn replace_modifier_should_replace_a_string() {
        // Arrange
        let value_modifier =
            ValueModifier::ReplaceAll { find: "Hello".to_owned(), replace: "World".to_owned() };

        // Act & Assert
        {
            let mut input = Value::Text("Hello World".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("World World".to_owned()), input);
        }
    }

    #[test]
    fn to_number_modifier_should_return_a_number() {
        // Arrange
        let value_modifier = ValueModifier::ToNumber;

        // Act & Assert
        {
            let mut input = Value::Text("12".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Number(Number::PosInt(12)), input);
        }

        {
            let mut input = Value::Text("-3412".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Number(Number::NegInt(-3412)), input);
        }

        {
            let mut input = Value::Text("3.14".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Number(Number::Float(3.14)), input);
        }

        {
            let mut input = Value::Text("something".to_owned());
            assert!(value_modifier.apply("", &mut input).is_err());
        }
    }

    #[test]
    fn replace_with_regex_modifier_should_replace_a_string() {
        // Arrange
        let value_modifier = ValueModifier::ReplaceAllRegex {
            find_regex: RegexWrapper::new("[0-9]+").unwrap(),
            replace: "number".to_owned(),
        };

        // Act & Assert
        {
            let mut input = Value::Text("Hello World 123 4!".to_owned());
            value_modifier.apply("", &mut input).unwrap();
            assert_eq!(Value::Text("Hello World number number!".to_owned()), input);
        }
    }
}
