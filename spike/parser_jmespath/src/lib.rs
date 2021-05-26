use jmespath::{Rcvar, ToJmespath};
use std::borrow::Cow;
use thiserror::Error;
use tornado_common_api::{Number, Payload, Value};

pub const EXPRESSION_START_DELIMITER: &str = "${";
pub const EXPRESSION_END_DELIMITER: &str = "}";

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("ConfigurationError: [{message}]")]
    ConfigurationError { message: String },
    #[error("ParsingError: [{message}]")]
    ParsingError { message: String },
}

#[derive(PartialEq, Debug)]
pub enum Parser {
    Exp(jmespath::Expression<'static>),
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
            let jmespath_exp =
                jmespath::compile(expression).map_err(|err| ParserError::ConfigurationError {
                    message: format!("Not valid expression: [{}]. Err: {:?}", expression, err),
                })?;
            Ok(Parser::Exp(jmespath_exp))
        } else {
            Ok(Parser::Val(Value::Text(text.to_owned())))
        }
    }

    pub fn parse_str<'o>(&'o self, value: &str) -> Result<Cow<'o, Value>, ParserError> {
        let data: Value =
            serde_json::from_str(value).map_err(|err| ParserError::ConfigurationError {
                message: format!("Failed to parse str into Value. Err: {:?}", err),
            })?;
        self.parse_value(&data)
    }

    pub fn parse_value<'o>(&'o self, value: &Value) -> Result<Cow<'o, Value>, ParserError> {
        match self {
            Parser::Exp(exp) => search(exp, value).map(Cow::Owned),
            Parser::Val(value) => Ok(Cow::Borrowed(value)),
        }
    }
}

fn search<T: ToJmespath>(
    exp: &jmespath::Expression<'static>,
    data: T,
) -> Result<Value, ParserError> {
    let search_result = exp.search(data).map_err(|e| ParserError::ParsingError {
        message: format!("Expression failed to execute. Exp: {}. Error: {}", exp, e),
    })?;
    variable_to_value(&search_result)
}

fn variable_to_value(var: &Rcvar) -> Result<Value, ParserError> {
    match var.as_ref() {
        jmespath::Variable::String(s) => Ok(Value::Text(s.to_owned())),
        jmespath::Variable::Bool(b) => Ok(Value::Bool(*b)),
        jmespath::Variable::Number(n) => {
            Ok(Value::Number(Number::from_serde_number(n).ok_or_else(|| {
                ParserError::ParsingError {
                    message: "Cannot map jmespath::Variable::Number to a Value::Number".to_owned(),
                }
            })?))
        }
        jmespath::Variable::Object(values) => {
            let mut payload = Payload::new();
            for (key, value) in values {
                payload.insert(key.to_owned(), variable_to_value(value)?);
            }
            Ok(Value::Map(payload))
        }
        jmespath::Variable::Array(ref values) => {
            let mut payload = vec![];
            for value in values {
                payload.push(variable_to_value(value)?);
            }
            Ok(Value::Array(payload))
        }
        jmespath::Variable::Null => Ok(Value::Null),
        // ToDo: how to map Expref?
        jmespath::Variable::Expref(_) => Err(ParserError::ParsingError {
            message: "Cannot map jmespath::Variable::Expref to the Event payload".to_owned(),
        }),
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;

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
            Parser::Exp(exp) => {
                assert_eq!(exp.as_str(), "hello.world");
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
        let result = parser.parse_str(&json);

        // Assert
        assert!(result.is_ok());
        assert_eq!(&Value::Text("hello world".to_owned()), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_return_from_json() {
        // Arrange
        let exp = jmespath::compile("level_one.level_two").unwrap();
        let parser = Parser::Exp(exp);
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;

        // Act
        let result = parser.parse_str(json);

        // Assert
        assert!(result.is_ok());
        assert_eq!(&Value::Text("level_two_value".to_owned()), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_return_null_if_not_present() {
        // Arrange
        let exp = jmespath::compile("level_one.level_three").unwrap();
        let parser = Parser::Exp(exp);
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;

        // Act
        let result = parser.parse_str(json);

        // Assert
        assert!(result.is_ok());
        assert_eq!(&Value::Null, result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_return_null_if_not_present_in_array() {
        // Arrange
        let exp = jmespath::compile("level_one.level_two[2]").unwrap();
        let parser = Parser::Exp(exp);
        let json = r#"
        {
            "level_one": {
                "level_two": ["level_two_0", "level_two_1"]
            }
        }
        "#;

        // Act
        let result = parser.parse_str(json);

        // Assert
        assert!(result.is_ok());
        assert_eq!(&Value::Null, result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_handle_boolean_values() {
        // Arrange
        let exp = jmespath::compile("key").unwrap();
        let parser = Parser::Exp(exp);
        let json = r#"
        {
            "key": true
        }
        "#;

        // Act
        let result = parser.parse_str(json);

        // Assert
        assert!(result.is_ok());
        assert_eq!(&Value::Bool(true), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_handle_numeric_values() {
        // Arrange
        let exp = jmespath::compile("key").unwrap();
        let parser = Parser::Exp(exp);
        let json = r#"
        {
            "key": 99.66
        }
        "#;

        // Act
        let result = parser.parse_str(json);

        // Assert
        assert!(result.is_ok());
        assert_eq!(&Value::Number(Number::Float(99.66)), result.unwrap().as_ref());
    }

    #[test]
    fn parser_expression_should_handle_arrays() {
        // Arrange
        let exp = jmespath::compile("key").unwrap();
        let parser = Parser::Exp(exp);
        let json = r#"
        {
            "key": ["one", true, 13]
        }
        "#;

        let value: Value = serde_json::from_str(json).unwrap();

        // Act
        let result = parser.parse_value(&value);

        // Assert
        assert!(result.is_ok());
        assert_eq!(
            &Value::Array(vec![
                Value::Text("one".to_owned()),
                Value::Bool(true),
                Value::Number(Number::PosInt(13))
            ]),
            result.unwrap().as_ref()
        );
    }

    #[test]
    fn parser_expression_should_handle_maps() {
        // Arrange
        let exp = jmespath::compile("key").unwrap();
        let parser = Parser::Exp(exp);
        let json = r#"
        {
            "key": {
                "one": true,
                "two": 13.0
            }
        }
        "#;

        // Act
        let result = parser.parse_str(&json);

        // Assert
        assert!(result.is_ok());

        let mut payload = HashMap::new();
        payload.insert("one".to_owned(), Value::Bool(true));
        payload.insert("two".to_owned(), Value::Number(Number::Float(13 as f64)));

        assert_eq!(&Value::Map(payload), result.unwrap().as_ref());
    }

    #[test]
    fn parser_should_enable_jmespath_functions() {
        // Arrange
        let parser = Parser::build_parser("${contains(@, 'one')}").unwrap();
        let json1 = r#"["one", "two"]"#;
        let json2 = r#"["three", "four"]"#;

        // Act
        let result1 = parser.parse_str(&json1);
        let result2 = parser.parse_str(&json2);

        // Assert
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        assert_eq!(&Value::Bool(true), result1.unwrap().as_ref());
        assert_eq!(&Value::Bool(false), result2.unwrap().as_ref());
    }
}
