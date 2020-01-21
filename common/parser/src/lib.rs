use failure_derive::Fail;
use lazy_static::*;
use regex::Regex;
use std::borrow::Cow;
use tornado_common_api::Value;
use crate::interpolator::StringInterpolator;

mod interpolator;

pub const EXPRESSION_START_DELIMITER: &str = "${";
pub const EXPRESSION_END_DELIMITER: &str = "}";
const PAYLOAD_KEY_PARSE_REGEX: &str = r#"("[^"]+"|[^\.^\[]+|\[[^\]]+\])"#;
const PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER: char = '"';
const PAYLOAD_ARRAY_KEY_START_DELIMITER: char = '[';
const PAYLOAD_ARRAY_KEY_END_DELIMITER: char = ']';

lazy_static! {
    static ref RE: Regex = Regex::new(PAYLOAD_KEY_PARSE_REGEX).expect("Parser regex must be valid");
}

#[derive(Fail, Debug)]
pub enum ParserError {
    #[fail(display = "ConfigurationError: [{}]", message)]
    ConfigurationError { message: String },
    #[fail(display = "ParsingError: [{}]", message)]
    ParsingError { message: String },
    #[fail(
    display = "InterpolatorRenderError: Cannot resolve placeholders in template [{}] cause: [{}]",
    template, cause
    )]
    InterpolatorRenderError { template: String, cause: String },
}

#[derive(PartialEq, Debug)]
pub enum Parser {
    Exp { keys: Vec<ValueGetter> },
    Interpolator { interpolator: StringInterpolator },
    Val(Value),
}

impl Parser {
    pub fn is_expression(text: &str) -> bool {
        let trimmed = text.trim();
        trimmed.starts_with(EXPRESSION_START_DELIMITER)
            && trimmed.ends_with(EXPRESSION_END_DELIMITER)
    }

    pub fn build_parser(text: &str) -> Result<Parser, ParserError> {
        if Parser::is_expression(text) {
            let trimmed = text.trim();
            let expression = &trimmed[EXPRESSION_START_DELIMITER.len()
                ..(trimmed.len() - EXPRESSION_END_DELIMITER.len())];
            Ok(Parser::Exp { keys: Parser::parse_keys(expression)? })
        } else {
            let interpolator = StringInterpolator::build(text)?;
            if interpolator.is_interpolation_required() {
                Ok(Parser::Interpolator{interpolator})
            } else {
                Ok(Parser::Val(Value::Text(text.to_owned())))
            }
        }
    }

    fn parse_keys(expression: &str) -> Result<Vec<ValueGetter>, ParserError> {
        let regex: &Regex = &RE;
        regex
            .captures_iter(expression)
            .map(|cap| {
                let capture = cap.get(0).ok_or_else(|| ParserError::ConfigurationError {
                    message: format!("Error parsing expression [{}]", expression),
                })?;
                let mut result = capture.as_str().to_string();

                // Remove trailing delimiters
                {
                    if result.starts_with(PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER)
                        && result.ends_with(PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER)
                    {
                        result = result[1..(result.len() - 1)].to_string();
                    }
                    if result.starts_with(PAYLOAD_ARRAY_KEY_START_DELIMITER)
                        && result.ends_with(PAYLOAD_ARRAY_KEY_END_DELIMITER)
                    {
                        result = result[1..(result.len() - 1)].to_string();
                        let index = usize::from_str_radix(&result, 10).map_err(|err| {
                            ParserError::ConfigurationError {
                                message: format!(
                                    "Cannot parse value [{}] to number: {}",
                                    &result, err
                                ),
                            }
                        })?;
                        return Ok(ValueGetter::Array { index });
                    }
                    if result.contains(PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER) {
                        let error_message = format!(
                            "Parser expression [{}] contains not valid characters: [{}]",
                            expression, PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER
                        );
                        return Err(ParserError::ConfigurationError { message: error_message });
                    }
                }
                Ok(ValueGetter::Map { key: result })
            })
            .collect()
    }

    pub fn parse_value<'o>(&'o self, value: &'o Value) -> Option<Cow<'o, Value>> {
        match self {
            Parser::Exp { keys } => {
                let mut temp_value = Some(value);

                let mut count = 0;

                while count < keys.len() && temp_value.is_some() {
                    temp_value = temp_value.and_then(|val| keys[count].get(val));
                    count += 1;
                }

                temp_value.map(|value| Cow::Borrowed(value))
            },
            Parser::Interpolator {interpolator} => interpolator.render(value).map(|text| Cow::Owned(Value::Text(text))).ok(),
            Parser::Val(value) => Some(Cow::Borrowed(value)),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ValueGetter {
    Map { key: String },
    Array { index: usize },
}

impl ValueGetter {
    pub fn get<'o>(&self, value: &'o Value) -> Option<&'o Value> {
        match self {
            ValueGetter::Map { key } => value.get_from_map(key),
            ValueGetter::Array { index } => value.get_from_array(*index),
        }
    }
}

impl Into<ValueGetter> for &str {
    fn into(self) -> ValueGetter {
        ValueGetter::Map { key: self.to_owned() }
    }
}

impl Into<ValueGetter> for usize {
    fn into(self) -> ValueGetter {
        ValueGetter::Array { index: self }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;
    use tornado_common_api::Number;

    #[test]
    fn parser_builder_should_return_value_type() {
        // Act
        let parser = Parser::build_parser("hello world").unwrap();

        // Assert
        match parser {
            Parser::Val(value) => {
                assert_eq!(Value::Text("hello world".to_owned()), value);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn parser_builder_should_return_value_exp() {
        // Act
        let parser = Parser::build_parser("${hello.world}").unwrap();

        // Assert
        match parser {
            Parser::Exp { keys } => {
                assert!(!keys.is_empty());
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn parser_text_should_return_static_text() {
        // Arrange
        let parser = Parser::Val(Value::Text("hello world".to_owned()));
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;

        // Act
        let value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value);

        // Assert
        assert!(result.is_some());
        assert_eq!(&Value::Text("hello world".to_owned()), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_return_from_json() {
        // Arrange
        let parser = Parser::build_parser("${level_one.level_two}").unwrap();
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;

        // Act
        let value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value);

        // Assert
        assert!(result.is_some());
        assert_eq!(&Value::Text("level_two_value".to_owned()), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_return_none_if_not_present() {
        // Arrange
        let parser = Parser::build_parser("${level_one.level_three}").unwrap();
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;

        // Act
        let value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn parser_expression_should_return_none_if_not_present_in_array() {
        // Arrange
        let parser = Parser::build_parser("${level_one.level_two[2]}").unwrap();
        let json = r#"
        {
            "level_one": {
                "level_two": ["level_two_0", "level_two_1"]
            }
        }
        "#;

        // Act
        let value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn parser_expression_should_handle_boolean_values() {
        // Arrange
        let parser = Parser::build_parser("${key}").unwrap();
        let json = r#"
        {
            "key": true
        }
        "#;

        // Act
        let value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value);

        // Assert
        assert!(result.is_some());
        assert_eq!(&Value::Bool(true), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_handle_numeric_values() {
        // Arrange
        let parser = Parser::build_parser("${key}").unwrap();
        let json = r#"
        {
            "key": 99.66
        }
        "#;

        // Act
        let value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value);

        // Assert
        assert!(result.is_some());
        assert_eq!(&Value::Number(Number::Float(99.66)), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_handle_arrays() {
        // Arrange
        let parser = Parser::build_parser("${key}").unwrap();
        let json = r#"
        {
            "key": ["one", true, 13.0]
        }
        "#;

        let value: Value = serde_json::from_str(json).unwrap();

        // Act
        let result = parser.parse_value(&value);

        // Assert
        assert!(result.is_some());
        assert_eq!(
            &Value::Array(vec![
                Value::Text("one".to_owned()),
                Value::Bool(true),
                Value::Number(Number::Float(13 as f64))
            ]),
            result.unwrap().as_ref()
        );
    }

    #[test]
    fn parser_expression_should_handle_maps() {
        // Arrange
        let parser = Parser::build_parser("${key}").unwrap();
        let json = r#"
        {
            "key": {
                "one": true,
                "two": 13
            }
        }
        "#;

        // Act
        let value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value);

        // Assert
        assert!(result.is_some());

        let mut payload = HashMap::new();
        payload.insert("one".to_owned(), Value::Bool(true));
        payload.insert("two".to_owned(), Value::Number(Number::PosInt(13)));

        assert_eq!(&Value::Map(payload), result.unwrap().as_ref());
    }

    #[test]
    fn builder_should_parse_a_payload_key() {
        let expected: Vec<ValueGetter> = vec!["one".into()];
        assert_eq!(expected, Parser::parse_keys("one").unwrap());

        let expected: Vec<ValueGetter> = vec!["one".into(), "two".into()];
        assert_eq!(expected, Parser::parse_keys("one.two").unwrap());

        let expected: Vec<ValueGetter> = vec!["one".into(), "two".into()];
        assert_eq!(expected, Parser::parse_keys("one.two.").unwrap());

        let expected: Vec<ValueGetter> = vec!["one".into(), "".into()];
        assert_eq!(expected, Parser::parse_keys(r#"one."""#).unwrap());

        let expected: Vec<ValueGetter> = vec!["one".into(), "two".into(), "th ir.d".into()];
        assert_eq!(expected, Parser::parse_keys(r#"one.two."th ir.d""#).unwrap());

        let expected: Vec<ValueGetter> =
            vec!["th ir.d".into(), "a".into(), "fourth".into(), "two".into()];
        assert_eq!(expected, Parser::parse_keys(r#""th ir.d".a."fourth".two"#).unwrap());

        let expected: Vec<ValueGetter> =
            vec!["payload".into(), "oids".into(), "SNMPv2-SMI::enterprises.14848.2.1.1.6.0".into()];
        assert_eq!(
            expected,
            Parser::parse_keys(r#"payload.oids."SNMPv2-SMI::enterprises.14848.2.1.1.6.0""#)
                .unwrap()
        );
    }

    #[test]
    fn payload_key_parser_should_fail_if_key_contains_double_quotes() {
        // Act
        let result = Parser::parse_keys(r#"o"ne"#);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn payload_key_parser_should_fail_if_key_does_not_contain_both_trailing_and_ending_quotes() {
        // Act
        let result = Parser::parse_keys(r#"one."two"#);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn builder_parser_should_return_empty_vector_if_no_matches() {
        let expected: Vec<ValueGetter> = vec![];
        assert_eq!(expected, Parser::parse_keys("").unwrap())
    }

    #[test]
    fn builder_parser_should_return_empty_vector_if_single_dot() {
        let expected: Vec<ValueGetter> = vec![];
        assert_eq!(expected, Parser::parse_keys(".").unwrap())
    }

    #[test]
    fn builder_parser_should_return_ignore_trailing_dot() {
        let expected: Vec<ValueGetter> = vec!["hello".into(), "world".into()];
        assert_eq!(expected, Parser::parse_keys(".hello.world").unwrap())
    }

    #[test]
    fn builder_parser_should_not_return_array_reader_if_within_double_quotes() {
        let expected: Vec<ValueGetter> =
            vec!["hello".into(), "world[11]".into(), "inner".into(), 0.into()];
        assert_eq!(expected, Parser::parse_keys(r#"hello."world[11]".inner[0]"#).unwrap())
    }

    #[test]
    fn builder_parser_should_return_array_reader() {
        let expected: Vec<ValueGetter> =
            vec!["hello".into(), "world".into(), 11.into(), "inner".into(), 0.into()];
        assert_eq!(expected, Parser::parse_keys("hello.world[11].inner[0]").unwrap())
    }
}
