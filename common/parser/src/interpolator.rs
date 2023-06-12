//
// For performance considerations see:
// - https://lise-henry.github.io/articles/optimising_strings.html
// - https://users.rust-lang.org/t/fast-string-concatenation/4425
// - https://github.com/hoodie/concatenation_benchmarks-rs
//

use crate::{Parser, ParserBuilder, ParserError};
use lazy_static::*;
use regex::{Match, Regex};
use serde_json::Value;
use std::fmt::Debug;
use std::marker::PhantomData;
use tornado_common_api::ValueGet;

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"(\$\{[^}]+})").expect("StringInterpolator regex must be valid");
}

#[derive(Debug)]
pub struct StringInterpolator<T: Debug> {
    template: String,
    parsers: Vec<BoundedAccessor<T>>,
}

#[derive(Debug)]
struct BoundedAccessor<T: Debug> {
    start: usize,
    end: usize,
    parser: Parser<T>,
    _phantom: PhantomData<T>,
}

impl<T: Debug> StringInterpolator<T> {
    /// Creates a new StringInterpolator
    pub fn build<S: Into<String>>(
        template: S,
        parser_builder: &ParserBuilder<T>,
    ) -> Result<Option<Self>, ParserError> {
        let template_string = template.into();

        let regex: &Regex = &RE;
        let matchers = regex.find_iter(&template_string).collect::<Vec<Match<'_>>>();

        if StringInterpolator::<T>::interpolation_required(&template_string, &matchers) {
            let parsers = matchers
                .iter()
                .map(|m| {
                    parser_builder.build_parser(m.as_str()).map(|parser| BoundedAccessor {
                        start: m.start(),
                        end: m.end(),
                        parser,
                        _phantom: PhantomData,
                    })
                })
                .collect::<Result<Vec<_>, ParserError>>()?;

            return Ok(Some(StringInterpolator { template: template_string.clone(), parsers }));
        }
        Ok(None)
    }

    /// Returns whether the template used to create this StringInterpolator
    /// requires interpolation.
    /// This is true only if the template contains at least both a static part (e.g. constant text)
    /// and a dynamic part (e.g. placeholders to be resolved at runtime).
    /// When the interpolator is not required, it can be replaced by a simpler Accessor.
    fn interpolation_required(template: &str, matches: &[Match]) -> bool {
        matches.len() > 1
            || (matches.len() == 1
                && !(matches[0].start() == 0 && matches[0].end() == template.len()))
    }

    /// Performs the placeholders substitution on the internal template and return the
    /// resulting string.
    /// The placeholders are replaced with values extracted from the passed value.
    /// Only values of type String, Number, Boolean and Null are accepted; consequently, this method
    /// will return an error if:
    /// - the placeholder cannot be resolved
    /// - the value associated with the placeholder is of type Array
    /// - the value associated with the placeholder is of type Map
    pub fn render<I: ValueGet>(&self, event: &I, context: &T) -> Result<String, ParserError> {
        let mut render = String::new();

        // keeps the index of the previous argument end
        let mut prev_end = 0;

        for bounded_accessor in &self.parsers {
            if prev_end != bounded_accessor.start {
                render.push_str(&self.template[prev_end..bounded_accessor.start])
            }

            let accessor = &bounded_accessor.parser;

            let value = accessor.parse_value(event, context).ok_or(
                ParserError::InterpolatorRenderError {
                    template: self.template.to_owned(),
                    cause: format!("Accessor [{:?}] returned empty value.", accessor),
                },
            )?;
            match value.as_ref() {
                Value::String(text) => render.push_str(text.as_str()),
                Value::Bool(val) => render.push_str(&val.to_string()),
                Value::Number(val) => render.push_str(&val.to_string()),
                Value::Null => render.push_str("null"),
                Value::Object(..) => return Err(ParserError::InterpolatorRenderError {
                    template: self.template.to_owned(),
                    cause: format!("Accessor [{:?}] returned a Map. Expected text, number, boolean or null.", accessor),
                }),
                Value::Array(..) => return Err(ParserError::InterpolatorRenderError {
                    template: self.template.to_owned(),
                    cause: format!("Accessor [{:?}] returned an Array. Expected text, number, boolean or null.", accessor),
                }),
            }

            prev_end = bounded_accessor.end;
        }

        let template_len = self.template.len();

        // if last arg end index isn't the end of the string then copy
        // from last arg end till end of template
        if prev_end < template_len {
            render.push_str(&self.template[prev_end..template_len])
        }

        Ok(render)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Parser, ValueGetter};
    use serde_json::json;
    use tornado_common_api::Payload;

    #[test]
    fn should_create_new_interpolator() {
        // Arrange
        let template = "<div><span>${event.payload.test}</sp${event.type}an><span>${_variables.test12}</span></${}div>";

        // Act
        let interpolator =
            StringInterpolator::<()>::build(template, &ParserBuilder::default()).unwrap().unwrap();

        // Assert
        assert_eq!(3, interpolator.parsers.len());

        assert_eq!(&11, &interpolator.parsers[0].start);
        assert_eq!(&32, &interpolator.parsers[0].end);
        match &interpolator.parsers[0].parser {
            Parser::Exp { keys } => assert_eq!(3, keys.len()),
            _ => unreachable!(),
        }

        assert_eq!(&36, &interpolator.parsers[1].start);
        assert_eq!(&49, &interpolator.parsers[1].end);
        match &interpolator.parsers[1].parser {
            Parser::Exp { keys } => assert_eq!(2, keys.len()),
            _ => unreachable!(),
        }

        assert_eq!(&58, &interpolator.parsers[2].start);
        assert_eq!(&78, &interpolator.parsers[2].end);
        match &interpolator.parsers[2].parser {
            Parser::Exp { keys } => {
                assert_eq!(2, keys.len());
                assert_eq!(ValueGetter::Map { key: "_variables".to_owned() }, keys[0]);
                assert_eq!(ValueGetter::Map { key: "test12".to_owned() }, keys[1]);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn should_return_none_with_no_expressions_delimiters() {
        // Arrange
        let template = "constant string";

        // Act
        let result = StringInterpolator::<()>::build(template, &ParserBuilder::default()).unwrap();

        // Assert
        assert!(result.is_none());
    }
    #[test]
    fn should_return_none_with_single_expression() {
        // Arrange
        let template = "${event.type}";

        // Act
        let result = StringInterpolator::<()>::build(template, &ParserBuilder::default()).unwrap();

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn should_split_with_only_expressions() {
        // Arrange
        let template = "${event.type}${event.created_ms}${event.type}";

        // Act
        let interpolator =
            StringInterpolator::<()>::build(template, &ParserBuilder::default()).unwrap().unwrap();

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
            StringInterpolator::build(template, &ParserBuilder::default()).unwrap().unwrap();
        let result = interpolator.render(&Value::Object(payload), &());

        // Assert
        assert!(result.is_ok());
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
            StringInterpolator::build(template, &ParserBuilder::default()).unwrap().unwrap();
        let result = interpolator.render(&Value::Object(payload), &());

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
            StringInterpolator::build(template, &ParserBuilder::default()).unwrap().unwrap();
        let result = interpolator.render(&Value::Object(payload), &());

        // Assert
        assert!(result.is_ok());
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
            StringInterpolator::build(template, &ParserBuilder::default()).unwrap().unwrap();
        let result = interpolator.render(&Value::Object(payload), &());

        // Assert
        assert!(result.is_ok());
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
            StringInterpolator::build(template, &ParserBuilder::default()).unwrap().unwrap();
        let result = interpolator.render(&Value::Object(payload), &());

        // Assert
        assert!(result.is_err());
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
            StringInterpolator::build(template, &ParserBuilder::default()).unwrap().unwrap();
        let result = interpolator.render(&Value::Object(payload), &());

        // Assert
        assert!(result.is_err());
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
            StringInterpolator::build(template, &ParserBuilder::default()).unwrap().unwrap();
        let result = interpolator.render(&Value::Object(payload), &());

        // Assert
        assert!(result.is_err());
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
            StringInterpolator::build(template, &ParserBuilder::default()).unwrap().unwrap();
        let result = interpolator.render(&event, &());

        // Assert
        assert!(result.is_ok());

        println!("---------------------------");
        println!("Event: \n{:?}", event);
        println!("Template: \n{}", template);
        println!("Rendered template: \n{}", result.unwrap());
        println!("---------------------------");
    }
}
