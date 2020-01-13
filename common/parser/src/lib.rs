use failure_derive::Fail;
use jmespath::{Rcvar, ToJmespath};
use tornado_common_api::{Value, Payload, Number};
use std::borrow::Cow;

const EXPRESSION_START_DELIMITER: &str = "${";
const EXPRESSION_END_DELIMITER: &str = "}";

#[derive(Fail, Debug)]
pub enum ParserError {
    #[fail(display = "ConfigurationError: [{}]", message)]
    ConfigurationError { message: String },
    #[fail(display = "ParsingError: [{}]", message)]
    ParsingError { message: String },
}

pub enum Parser {
    Exp(jmespath::Expression<'static>),
    Val(Value)
}

impl Parser {

    pub fn build_parser(text: &str) -> Result<Parser, ParserError> {
        if text.starts_with(EXPRESSION_START_DELIMITER) && text.ends_with(EXPRESSION_END_DELIMITER)
        {
            let expression = &text
                [EXPRESSION_START_DELIMITER.len()..(text.len() - EXPRESSION_END_DELIMITER.len())];
            let jmespath_exp = jmespath::compile(expression).map_err(|err| {
                ParserError::ConfigurationError {
                    message: format!(
                        "Not valid expression: [{}]. Err: {}",
                        expression, err
                    ),
                }
            })?;
            Ok(Parser::Exp(jmespath_exp))
        } else {
            Ok(Parser::Val(Value::Text(text.to_owned())))
        }
    }

    pub fn parse_str<'o>(
        &'o self,
        value: &'o str,
    ) -> Result<Cow<'o, Value>, ParserError> {
        match self {
            Parser::Exp(exp) => search(exp, value).map(|val| Cow::Owned(val)),
            Parser::Val(value) => Ok(Cow::Borrowed(value))
        }
    }

    pub fn parse_value<'o>(
        &'o self,
        value: &'o Value,
    ) -> Result<Cow<'o, Value>, ParserError> {
        match self {
            Parser::Exp(exp) => search(exp, value).map(|val| Cow::Owned(val)),
            Parser::Val(value) => Ok(Cow::Borrowed(value))
        }
    }

}

fn search<T: ToJmespath>(exp: &jmespath::Expression<'static>, data: T) -> Result<Value, ParserError> {
    let search_result = exp.search(data).map_err(|e| ParserError::ParsingError {
        message: format!(
            "Expression failed to execute. Exp: {}. Error: {}",
            exp, e
        ),
    })?;
    variable_to_value(&search_result)
}

fn variable_to_value(var: &Rcvar) -> Result<Value, ParserError> {
    match var.as_ref() {
        jmespath::Variable::String(s) => Ok(Value::Text(s.to_owned())),
        jmespath::Variable::Bool(b) => Ok(Value::Bool(*b)),
        jmespath::Variable::Number(n) => Ok(Value::Number(Number::Float(*n))),
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