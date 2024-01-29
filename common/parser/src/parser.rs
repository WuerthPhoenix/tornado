use crate::interpolator::StringInterpolator;
use crate::{CustomParser, Template, ValueGetter, FOREACH_ITEM_KEY};
use lazy_static::*;
use regex::Regex;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::identity;
use std::fmt::Debug;
use thiserror::Error;
use tornado_common_types::ValueGet;

pub const EXPRESSION_NESTED_DELIMITER: &str = ".";
const PAYLOAD_KEY_PARSE_REGEX: &str = r#"("[^"]+"|[^\.^\[]+|\[[^\]]+\])"#;
const PAYLOAD_MAP_KEY_PARSE_TRAILING_DELIMITER: char = '"';
const PAYLOAD_ARRAY_KEY_START_DELIMITER: char = '[';
const PAYLOAD_ARRAY_KEY_END_DELIMITER: char = ']';
pub const EXTRACTED_VARIABLES_KEY: &str = "_variables";

lazy_static! {
    static ref RE: Regex = Regex::new(PAYLOAD_KEY_PARSE_REGEX).expect("Parser regex must be valid");
}

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("ConfigurationError: [{message}]")]
    ConfigurationError { message: String },
    #[error("UnknownKeyError: [{key}]")]
    UnknownKeyError { key: String },
}

pub trait ParserFactory {
    fn build(&self, expression: &[ValueGetter]) -> Result<Box<dyn CustomParser>, ParserError>;
}

impl<F: Fn(&[ValueGetter]) -> Result<Box<dyn CustomParser>, ParserError>> ParserFactory for F {
    fn build(&self, expression: &[ValueGetter]) -> Result<Box<dyn CustomParser>, ParserError> {
        self(expression)
    }
}

#[derive(Default)]
pub struct ParserBuilder {
    custom_parser_factories: HashMap<String, Box<dyn ParserFactory>>,
    ignored_expressions: Vec<String>,
}

impl ParserBuilder {
    pub fn add_parser_factory(mut self, key: String, factory: Box<dyn ParserFactory>) -> Self {
        self.custom_parser_factories.insert(key, factory);
        self
    }

    pub fn add_ignored_expression(mut self, field: String) -> Self {
        self.ignored_expressions.push(field);
        self
    }

    pub fn is_ignored_extractor(&self, extractor: &str) -> bool {
        extractor
            .strip_prefix("${")
            .and_then(|rest| rest.strip_suffix('}'))
            .map(|rest| {
                self.ignored_expressions
                    .iter()
                    .map(|expr| key_is_root_entry_of_expression(expr, rest))
                    .any(identity)
            })
            .unwrap_or(false)
    }

    pub fn engine_matcher() -> ParserBuilder {
        ParserBuilder::default()
            .add_parser_factory(
                EXTRACTED_VARIABLES_KEY.to_owned(),
                Box::new(ExtractedVarParser::try_new),
            )
            .add_ignored_expression(FOREACH_ITEM_KEY.to_owned())
    }

    pub fn build_parser(&self, template_string: &str) -> Result<Parser, ParserError> {
        let template = Template::from(template_string);

        if template.is_interpolator() {
            Ok(Parser::Interpolator { interpolator: StringInterpolator::build(template, self)? })
        } else if template.is_accessor() && !self.is_ignored_extractor(template_string) {
            self.parse_expression(template_string)
        } else {
            Ok(Parser::Val(Value::String(template_string.to_owned())))
        }
    }

    pub fn parse_expression(&self, keys: &str) -> Result<Parser, ParserError> {
        let expression = &keys[2..keys.len() - 1];

        let getters = Parser::parse_keys(expression)?;
        let (head, tail) = match getters.as_slice() {
            [] // "${}"
            | [ValueGetter::Map { .. }] // "${event}"
            | [ValueGetter::Array { .. }, ..] // "${[123]event}"
            | [ValueGetter::Map { .. }, ValueGetter::Array { .. }, ..] => { // "${event[123]}"
                return Ok(Parser::Exp { keys: getters })
            }
            [ValueGetter::Map { key }, tail @ ..] => (key, tail), // "${event.timestamp}"
        };

        for (key, factory) in &self.custom_parser_factories {
            if key == head {
                return Ok(Parser::Custom {
                    key: ValueGetter::Map { key: head.to_owned() },
                    parser: factory.build(tail)?,
                });
            }
        }

        Ok(Parser::Exp { keys: getters })
    }
}

#[derive(Debug)]
pub enum Parser {
    Exp { keys: Vec<ValueGetter> },
    Interpolator { interpolator: StringInterpolator },
    Val(Value),
    Custom { key: ValueGetter, parser: Box<dyn CustomParser> },
}

impl Parser {
    fn parse_keys(expression: &str) -> Result<Vec<ValueGetter>, ParserError> {
        RE.captures_iter(expression)
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
                        let index =
                            result.parse().map_err(|err| ParserError::ConfigurationError {
                                message: format!(
                                    "Cannot parse value [{}] to number: {}",
                                    &result, err
                                ),
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

    pub fn parse_value<'o, I: ValueGet>(
        &'o self,
        value: &'o I,
        context: &str,
    ) -> Option<Cow<'o, Value>> {
        match self {
            Parser::Exp { keys } => {
                let mut key_iter = keys.iter();
                if let Some(key) = key_iter.next() {
                    let mut temp_value = key.get(value);
                    while let (Some(key), Some(val)) = (key_iter.next(), temp_value) {
                        temp_value = key.get(val);
                    }
                    temp_value.map(Cow::Borrowed)
                } else {
                    None
                }
            }
            Parser::Interpolator { interpolator } => {
                interpolator.render(value, context).map(|text| Cow::Owned(Value::String(text)))
            }
            Parser::Val(value) => Some(Cow::Borrowed(value)),
            Parser::Custom { key, parser } => {
                key.get(value).and_then(|val| parser.parse_value(val, context))
            }
        }
    }
}

/// Determines if a key is the first part of an expression.
///
/// # Example:
///
/// ``` Rust
/// assert!(key_is_root_entry_of_expression("mykey", "mykey"))
/// assert!(key_is_root_entry_of_expression("mykey", "mykey[0]"))
/// assert!(key_is_root_entry_of_expression("mykey", "mykey.somefield.something"))
/// assert!(!key_is_root_entry_of_expression("mykey", "mykeys,.somefield.something"))
/// assert!(!key_is_root_entry_of_expression("mykeys", "mykey.somefield.something"))
/// ```
pub fn key_is_root_entry_of_expression(key: &str, expression: &str) -> bool {
    expression
        .strip_prefix(key)
        .map(|rest| {
            rest.is_empty()
                || rest.starts_with(EXPRESSION_NESTED_DELIMITER)
                || rest.starts_with(PAYLOAD_ARRAY_KEY_START_DELIMITER)
        })
        .unwrap_or(false)
}

/// Determines if a key is the first part of an expression.
///
/// # Example:
///
/// ``` Rust
/// assert!(key_is_root_entry_of_expression("mykey", "mykey"))
/// assert!(key_is_root_entry_of_expression("mykey", "mykey[0]"))
/// assert!(key_is_root_entry_of_expression("mykey", "mykey.somefield.something"))
/// assert!(!key_is_root_entry_of_expression("mykey", "mykeys,.somefield.something"))
/// assert!(!key_is_root_entry_of_expression("mykeys", "mykey.somefield.something"))
/// ```
pub fn key_is_object_root_entry_of_expression(key: &str, expression: &str) -> bool {
    expression
        .strip_prefix(key)
        .map(|rest| rest.is_empty() || rest.starts_with(EXPRESSION_NESTED_DELIMITER))
        .unwrap_or(false)
}

#[derive(Debug)]
pub struct ExtractedVarParser {
    parser: Parser,
}

impl ExtractedVarParser {
    pub fn try_new(expression: &[ValueGetter]) -> Result<Box<dyn CustomParser>, ParserError> {
        let parser = Parser::Exp { keys: expression.to_vec() };
        Ok(Box::new(ExtractedVarParser { parser }))
    }
}

impl CustomParser for ExtractedVarParser {
    fn parse_value<'o>(&'o self, value: &'o Value, context: &str) -> Option<Cow<'o, Value>> {
        value
            .get_from_map(context)
            .and_then(|rule_vars| self.parser.parse_value(rule_vars, context))
            .or_else(|| self.parser.parse_value(value, context))
    }
}

#[cfg(test)]
mod test {

    use serde_json::{json, Map};

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn parser_builder_should_return_value_type() {
        // Act
        let parser = ParserBuilder::default().build_parser("  hello world  ").unwrap();

        // Assert
        match parser {
            Parser::Val(value) => {
                assert_eq!(Value::String("  hello world  ".to_owned()), value);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parser_builder_should_return_value_exp() {
        // Act
        let parser = ParserBuilder::default().build_parser("${hello.world}").unwrap();

        // Assert
        match parser {
            Parser::Exp { keys } => {
                assert!(!keys.is_empty());
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parser_text_should_return_static_text() {
        // Arrange
        let parser = Parser::Val(Value::String("hello world".to_owned()));
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;

        // Act
        let value: Value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value, "");

        // Assert
        assert!(result.is_some());
        assert_eq!(&Value::String("hello world".to_owned()), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_return_from_json() {
        // Arrange
        let parser = ParserBuilder::default().build_parser("${level_one.level_two}").unwrap();
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;

        // Act
        let value: Value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value, "");

        // Assert
        assert!(result.is_some());
        assert_eq!(&Value::String("level_two_value".to_owned()), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_return_none_if_not_present() {
        // Arrange
        let parser = ParserBuilder::default().build_parser("${level_one.level_three}").unwrap();
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;

        // Act
        let value: Value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value, "");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn parser_expression_should_return_none_if_not_present_in_array() {
        // Arrange
        let parser = ParserBuilder::default().build_parser("${level_one.level_two[2]}").unwrap();
        let json = r#"
        {
            "level_one": {
                "level_two": ["level_two_0", "level_two_1"]
            }
        }
        "#;

        // Act
        let value: Value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value, "");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn parser_expression_should_handle_boolean_values() {
        // Arrange
        let parser = ParserBuilder::default().build_parser("${key}").unwrap();
        let json = r#"
        {
            "key": true
        }
        "#;

        // Act
        let value: Value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value, "");

        // Assert
        assert!(result.is_some());
        assert_eq!(&Value::Bool(true), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_handle_numeric_values() {
        // Arrange
        let parser = ParserBuilder::default().build_parser("${key}").unwrap();
        let json = r#"
        {
            "key": 99.66
        }
        "#;

        // Act
        let value: Value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value, "");

        // Assert
        assert!(result.is_some());
        assert_eq!(&json!(99.66), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_handle_arrays() {
        // Arrange
        let parser = ParserBuilder::default().build_parser("${key}").unwrap();
        let json = r#"
        {
            "key": ["one", true, 13.0]
        }
        "#;

        let value: Value = serde_json::from_str(json).unwrap();

        // Act
        let result = parser.parse_value(&value, "");

        // Assert
        assert!(result.is_some());
        assert_eq!(
            &Value::Array(vec![Value::String("one".to_owned()), Value::Bool(true), json!(13_f64)]),
            result.unwrap().as_ref()
        );
    }

    #[test]
    fn parser_expression_should_handle_maps() {
        // Arrange
        let parser = ParserBuilder::default().build_parser("${key}").unwrap();
        let json = r#"
        {
            "key": {
                "one": true,
                "two": 13
            }
        }
        "#;

        // Act
        let value: Value = serde_json::from_str(json).unwrap();
        let result = parser.parse_value(&value, "");

        // Assert
        assert!(result.is_some());

        let mut payload = Map::new();
        payload.insert("one".to_owned(), Value::Bool(true));
        payload.insert("two".to_owned(), json!(13));

        assert_eq!(&Value::Object(payload), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_use_the_interpolators() {
        // Arrange
        let parser =
            ParserBuilder::default().build_parser("${key[0]} - ${key[1]} - ${key[2]}").unwrap();
        let json = r#"
        {
            "key": ["one", true, 13.0]
        }
        "#;

        let value: Value = serde_json::from_str(json).unwrap();

        // Act
        let result = parser.parse_value(&value, "");

        // Assert
        assert!(result.is_some());
        assert_eq!(&Value::String("one - true - 13".to_owned()), result.unwrap().as_ref());
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

    #[test]
    fn parser_expression_should_work_with_hashmaps() {
        // Arrange
        let parser =
            ParserBuilder::default().build_parser("${key[0]} - ${key[1]} - ${key[2]}").unwrap();

        let value =
            Value::Array(vec![Value::String("one".to_owned()), Value::Bool(true), json!(13.0)]);
        let mut map = HashMap::new();
        map.insert("key", &value);

        // Act
        let result = parser.parse_value(&map, "");

        // Assert
        assert!(result.is_some());
        assert_eq!(&Value::String("one - true - 13".to_owned()), result.unwrap().as_ref());
    }

    #[test]
    fn builder_should_register_and_use_a_custom_parser() {
        // Arrange
        let parser = ParserBuilder::default()
            .add_parser_factory("custom_key".to_owned(), Box::new(custom_parser))
            .build_parser("${custom_key.something.else}")
            .unwrap();

        let map = json!({
            "key": true,
            "custom_key": {
                "one": 1,
                "two": 2
            }
        });

        // Act
        let result = parser.parse_value(&map, "custom_context").unwrap();

        // Assert
        assert_eq!(&json!({"one": 1, "two": 2 }), result.as_ref());
    }

    #[test]
    fn builder_should_register_and_use_an_ignored_expressions_if_expression_is_equal_to_ignored() {
        // Arrange
        let parser = ParserBuilder::default().add_ignored_expression("ignored_expr".to_owned());

        // Assert
        assert!(parser.is_ignored_extractor("${ignored_expr}"))
    }

    #[test]
    fn builder_should_register_and_use_an_ignored_expressions_if_expression_starts_with_ignored() {
        // Arrange
        let parser = ParserBuilder::default()
            .add_ignored_expression("ignored_expr".to_owned())
            .build_parser("${ignored_expr.something}")
            .unwrap();

        let map = json!({
            "key": true,
        });

        // Act
        let result = parser.parse_value(&map, "custom_context").unwrap();

        // Assert
        assert_eq!(&json!("${ignored_expr.something}"), result.as_ref());
    }

    #[test]
    fn builder_should_register_and_use_an_ignored_expressions_if_interpolated() {
        // Arrange
        let parser = ParserBuilder::default()
            .add_ignored_expression("ignored_expr".to_owned())
            .build_parser("my ignored expression is ${ignored_expr.something}!!")
            .unwrap();

        let map = json!({
            "key": true,
        });

        // Act
        let result = parser.parse_value(&map, "custom_context").unwrap();

        // Assert
        assert_eq!(&json!("my ignored expression is ${ignored_expr.something}!!"), result.as_ref());
    }

    #[test]
    fn builder_should_register_and_use_an_ignored_expressions_if_accessed_as_array() {
        // Arrange
        let parser = ParserBuilder::default()
            .add_ignored_expression("ignored_expr".to_owned())
            .build_parser("my ignored expression is ${ignored_expr[0].something}!!")
            .unwrap();

        let map = json!({
            "key": true,
        });

        // Act
        let result = parser.parse_value(&map, "custom_context").unwrap();

        // Assert
        assert_eq!(
            &json!("my ignored expression is ${ignored_expr[0].something}!!"),
            result.as_ref()
        );
    }

    #[test]
    fn key_is_root_entry_of_expression_should_evaluate() {
        // Assert
        assert!(key_is_root_entry_of_expression("somekey", "somekey"));
        assert!(!key_is_root_entry_of_expression("somekey", "somekeyss"));
        assert!(key_is_root_entry_of_expression("somekey", "somekey.something"));
        assert!(key_is_root_entry_of_expression("somekey", "somekey[0]"));
        assert!(key_is_root_entry_of_expression("somekey", "somekey[0].something"));
        assert!(!key_is_root_entry_of_expression("somekey", "some[0].something"));
    }

    #[test]
    fn builder_should_evaluate_expression_if_not_ignored() {
        // Arrange
        let parser = ParserBuilder::default()
            .add_ignored_expression("ignored_expr".to_owned())
            .build_parser("${not_ignored_expr.something}")
            .unwrap();

        let map = json!({
            "key": true,
            "not_ignored_expr": {
              "something": 1,
            }
        });

        // Act
        let result = parser.parse_value(&map, "custom_context").unwrap();

        // Assert
        assert_eq!(&json!(1), result.as_ref());
    }

    #[derive(Debug)]
    pub struct MyParser {
        pub expression: Vec<ValueGetter>,
    }

    impl CustomParser for MyParser {
        fn parse_value<'o>(&'o self, value: &'o Value, context: &str) -> Option<Cow<'o, Value>> {
            assert_eq!("custom_context", context);
            Some(Cow::Borrowed(value))
        }
    }

    fn custom_parser(expression: &[ValueGetter]) -> Result<Box<dyn CustomParser>, ParserError> {
        println!("build custom parser with expression: [{:?}]", expression);
        Ok(Box::new(MyParser { expression: expression.to_vec() }))
    }
}
