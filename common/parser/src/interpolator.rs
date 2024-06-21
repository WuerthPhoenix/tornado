//
// For performance considerations see:
// - https://lise-henry.github.io/articles/optimising_strings.html
// - https://users.rust-lang.org/t/fast-string-concatenation/4425
// - https://github.com/hoodie/concatenation_benchmarks-rs
//

use crate::parser::{Parser, ParserBuilder, ParserError};
use crate::Template;
use serde_json::Value;
use std::fmt::Debug;
use tornado_common_types::ValueGet;

#[derive(Debug)]
pub struct StringInterpolator {
    template: String,
    parsers: Vec<BoundedAccessor>,
}

#[derive(Debug)]
struct BoundedAccessor {
    start: usize,
    end: usize,
    parser: Parser,
}

impl StringInterpolator {
    /// Creates a new StringInterpolator
    pub fn build(template: Template, parser_builder: &ParserBuilder) -> Result<Self, ParserError> {
        let parsers = template
            .matches()
            .iter()
            .filter(|&m| !parser_builder.is_ignored_expression(m.as_str()))
            .map(|m| {
                parser_builder.build_parser(m.as_str()).map(|parser| BoundedAccessor {
                    start: m.start(),
                    end: m.end(),
                    parser,
                })
            })
            .collect::<Result<Vec<_>, ParserError>>()?;

        Ok(StringInterpolator { template: template.template_string.to_owned(), parsers })
    }

    /// Performs the placeholders substitution on the internal template and return the
    /// resulting string.
    /// The placeholders are replaced with values extracted from the passed value.
    /// Only values of type String, Number, Boolean and Null are accepted; consequently, this method
    /// will return an error if:
    /// - the placeholder cannot be resolved
    /// - the value associated with the placeholder is of type Array
    /// - the value associated with the placeholder is of type Map
    pub fn render<I: ValueGet>(&self, event: &I, context: &str) -> Option<String> {
        let mut render = String::new();

        // keeps the index of the previous argument end
        let mut prev_end = 0;

        for bounded_accessor in &self.parsers {
            if prev_end != bounded_accessor.start {
                render.push_str(&self.template[prev_end..bounded_accessor.start])
            }

            let accessor = &bounded_accessor.parser;

            let value = accessor.parse_value(event, context)?;
            match value.as_ref() {
                Value::String(text) => render.push_str(text.as_str()),
                Value::Bool(val) => render.push_str(&val.to_string()),
                Value::Number(val) => render.push_str(&val.to_string()),
                Value::Null => render.push_str("null"),
                Value::Object(..) | Value::Array(..) => return None,
            }

            prev_end = bounded_accessor.end;
        }

        let template_len = self.template.len();

        // if last arg end index isn't the end of the string then copy
        // from last arg end till end of template
        if prev_end < template_len {
            render.push_str(&self.template[prev_end..template_len])
        }

        Some(render)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{AccessorExpression, Parser, ValueGetter};
    use serde_json::json;
    use tornado_common_types::Payload;

    #[test]
    fn should_create_new_interpolator() {
        // Arrange
        let template = "<div><span>${event.payload.test}</sp${event.type}an><span>${_variables.test12}</span></${}div>";

        // Act
        let interpolator =
            StringInterpolator::build(template.into(), &ParserBuilder::default()).unwrap();

        // Assert
        assert_eq!(3, interpolator.parsers.len());

        assert_eq!(&11, &interpolator.parsers[0].start);
        assert_eq!(&32, &interpolator.parsers[0].end);
        match &interpolator.parsers[0].parser {
            Parser::Exp(AccessorExpression { keys }) => assert_eq!(3, keys.len()),
            _ => unreachable!(),
        }

        assert_eq!(&36, &interpolator.parsers[1].start);
        assert_eq!(&49, &interpolator.parsers[1].end);
        match &interpolator.parsers[1].parser {
            Parser::Exp(AccessorExpression { keys }) => assert_eq!(2, keys.len()),
            _ => unreachable!(),
        }

        assert_eq!(&58, &interpolator.parsers[2].start);
        assert_eq!(&78, &interpolator.parsers[2].end);
        match &interpolator.parsers[2].parser {
            Parser::Exp(AccessorExpression { keys }) => {
                assert_eq!(2, keys.len());
                assert_eq!(ValueGetter::Map { key: "_variables".to_owned() }, keys[0]);
                assert_eq!(ValueGetter::Map { key: "test12".to_owned() }, keys[1]);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn should_split_with_only_expressions() {
        // Arrange
        let template = "${event.type}${event.created_ms}${event.type}";

        // Act
        let interpolator =
            StringInterpolator::build(template.into(), &ParserBuilder::default()).unwrap();

        // Assert
        assert_eq!(3, interpolator.parsers.len());

        assert_eq!(&0, &interpolator.parsers[0].start);
        assert_eq!(&13, &interpolator.parsers[0].end);

        assert_eq!(&13, &interpolator.parsers[1].start);
        assert_eq!(&32, &interpolator.parsers[1].end);

        assert_eq!(&32, &interpolator.parsers[2].start);
        assert_eq!(&45, &interpolator.parsers[2].end);
    }

    #[test]
    fn should_render_numbers() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("created_ms".to_owned(), json!("1554130814854"));

        let template = " ${created_ms} ";

        // Act
        let interpolator =
            StringInterpolator::build(template.into(), &ParserBuilder::default()).unwrap();
        let result = interpolator.render(&Value::Object(payload), "");

        // Assert
        assert!(result.is_some());
        let render = result.unwrap();

        assert_eq!(" 1554130814854 ", &render);
    }

    #[test]
    fn should_render_booleans() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("success".to_owned(), Value::Bool(true));
        payload.insert("fail".to_owned(), Value::Bool(false));

        let template = "success: ${success} - fail: ${fail}";

        // Act
        let interpolator =
            StringInterpolator::build(template.into(), &ParserBuilder::default()).unwrap();
        let result = interpolator.render(&Value::Object(payload), "");

        // Assert
        // assert!(result.is_ok());
        let render = result.unwrap();

        assert_eq!("success: true - fail: false", &render);
    }

    #[test]
    fn should_render_null() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("void".to_owned(), Value::Null);

        let template = " void:  ${void} ";

        // Act
        let interpolator =
            StringInterpolator::build(template.into(), &ParserBuilder::default()).unwrap();
        let result = interpolator.render(&Value::Object(payload), "");

        // Assert
        assert!(result.is_some());
        let render = result.unwrap();

        assert_eq!(" void:  null ", &render);
    }

    #[test]
    fn should_render_with_line_break() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("first".to_owned(), Value::String("first line".to_owned()));
        payload.insert("second".to_owned(), Value::String("second line".to_owned()));

        let template = "${first}\n${second}";

        // Act
        let interpolator =
            StringInterpolator::build(template.into(), &ParserBuilder::default()).unwrap();
        let result = interpolator.render(&Value::Object(payload), "");

        // Assert
        assert!(result.is_some());
        let render = result.unwrap();

        assert_eq!("first line\nsecond line", &render);
    }

    #[test]
    fn render_should_fail_if_no_value_is_present() {
        // Arrange
        let payload = Payload::new();

        let template = "val: ${second}";

        // Act
        let interpolator =
            StringInterpolator::build(template.into(), &ParserBuilder::default()).unwrap();
        let result = interpolator.render(&Value::Object(payload), "");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn render_should_fail_if_value_is_an_array() {
        // Arrange
        let body = vec![Value::String("".to_owned())];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Array(body));

        let template = "val: ${body}";

        // Act
        let interpolator =
            StringInterpolator::build(template.into(), &ParserBuilder::default()).unwrap();
        let result = interpolator.render(&Value::Object(payload), "");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn render_should_fail_if_value_is_a_map() {
        // Arrange
        let mut body = Payload::new();
        body.insert("".to_owned(), Value::String("".to_owned()));

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Object(body));

        let template = "val: ${body}";

        // Act
        let interpolator =
            StringInterpolator::build(template.into(), &ParserBuilder::default()).unwrap();
        let result = interpolator.render(&Value::Object(payload), "");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn interpolator_demo() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::String("payload content".to_owned()));

        let mut event = Payload::new();
        event.insert("type".to_owned(), Value::String("email".to_owned()));
        event.insert("created_ms".to_owned(), json!("1554130814854"));
        event.insert("payload".to_owned(), Value::Object(payload));

        let mut event_data = Payload::new();
        event_data.insert("event".to_owned(), Value::Object(event));

        let event = Value::Object(event_data);

        let template = r#"
            Received event with type: ${event.type}
            timestamp: ${event.created_ms}
            body content: ${event.payload.body}
         "#;

        // Act
        let interpolator =
            StringInterpolator::build(template.into(), &ParserBuilder::default()).unwrap();
        let result = interpolator.render(&event, "");

        println!("{result:?}");
        // Assert
        assert!(result.is_some());

        println!("---------------------------");
        println!("Event: \n{:?}", event);
        println!("Template: \n{}", template);
        println!("Rendered template: \n{}", result.unwrap());
        println!("---------------------------");
    }
}
