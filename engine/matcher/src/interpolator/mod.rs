//
// For performance considerations see:
// - https://lise-henry.github.io/articles/optimising_strings.html
// - https://users.rust-lang.org/t/fast-string-concatenation/4425
// - https://github.com/hoodie/concatenation_benchmarks-rs
//

use crate::accessor::{Accessor, AccessorBuilder};
use crate::error::MatcherError;
use crate::model::InternalEvent;
use lazy_static::*;
use regex::Regex;
use std::collections::HashMap;
use tornado_common_api::{Number, Value};

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"(\$\{[^}]+})").expect("StringInterpolator regex must be valid");
}

/// A StringInterpolator allows the dynamic substitution of placeholders in a string
/// with values extracted from an incoming event or from the extracted variables.
/// E.g.:
/// ```rust
///
/// use tornado_common_api::{Payload, Value, Event};
/// use tornado_engine_matcher::model::InternalEvent;
/// use tornado_engine_matcher::interpolator::StringInterpolator;
///
/// let mut payload = Payload::new();
/// payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
///
/// let event = InternalEvent::new(Event {
///     event_type: "event_type_value".to_owned(),
///     created_ms: 1554130814854,
///     payload,
/// });
///
/// let template = "type: ${event.type} - body: ${event.payload.body}";
///
/// let interpolator = StringInterpolator::build(template, "rule", &Default::default()).unwrap();
/// let render = interpolator.render(&event, None).unwrap();
///
/// assert_eq!("type: event_type_value - body: body_value", &render);
///
/// ```
#[derive(Debug, PartialEq)]
pub struct StringInterpolator {
    template: String,
    rule_name: String,
    accessors: Vec<BoundedAccessor>,
}

#[derive(Debug, PartialEq)]
struct BoundedAccessor {
    start: usize,
    end: usize,
    accessor: Accessor,
}

impl StringInterpolator {
    /// Creates a new StringInterpolator
    pub fn build<T: Into<String>, R: Into<String>>(
        template: T,
        rule_name: R,
        accessor_builder: &AccessorBuilder,
    ) -> Result<Self, MatcherError> {
        let template_string = template.into();
        let rule_name_string = rule_name.into();

        let accessors = RE
            .find_iter(&template_string)
            .map(|m| {
                accessor_builder
                    .build(&rule_name_string, m.as_str())
                    .map(|accessor| BoundedAccessor { start: m.start(), end: m.end(), accessor })
            })
            .collect::<Result<Vec<_>, MatcherError>>()?;

        Ok(StringInterpolator { template: template_string, rule_name: rule_name_string, accessors })
    }

    /// Returns whether the template used to create this StringInterpolator
    /// requires interpolation.
    /// This is true only if the template contains at least both a static part (e.g. constant text)
    /// and a dynamic part (e.g. placeholders to be resolved at runtime).
    /// When the interpolator is not required, it can be replaced by a simpler Accessor.
    pub fn is_interpolation_required(&self) -> bool {
        self.accessors.len() > 1
            || (self.accessors.len() == 1
                && !(self.accessors[0].start == 0 && self.accessors[0].end == self.template.len()))
    }

    /// Performs the placeholders substitution on the internal template and return the
    /// resulting string.
    /// The placeholders are replaced with values extracted from the passed event and extracted_vars.
    /// Only values of type String, Number, Boolean and null are accepted; consequently, this method
    /// will return an error if:
    /// - the placeholder cannot be resolved
    /// - the value associated with the placeholder is of type Array
    /// - the value associated with the placeholder is of type Map
    pub fn render(
        &self,
        event: &InternalEvent,
        extracted_vars: Option<&HashMap<String, HashMap<String, Value>>>,
    ) -> Result<String, MatcherError> {
        let mut render = String::new();

        // keeps the index of the previous argument end
        let mut prev_end = 0;

        for bounded_accessor in &self.accessors {
            if prev_end != bounded_accessor.start {
                render.push_str(&self.template[prev_end..bounded_accessor.start])
            }

            let accessor = &bounded_accessor.accessor;

            let value = accessor.get(event, extracted_vars).ok_or(
                MatcherError::InterpolatorRenderError {
                    template: self.template.to_owned(),
                    rule_name: self.rule_name.to_owned(),
                    cause: format!("Accessor [{:?}] returned empty value.", accessor),
                },
            )?;
            match value.as_ref() {
                Value::Text(text) => render.push_str(text),
                Value::Bool(val) => render.push_str(&val.to_string()),
                Value::Number(val) => match val {
                    Number::NegInt(num) => render.push_str(&num.to_string()),
                    Number::PosInt(num) => render.push_str(&num.to_string()),
                    Number::Float(num) => render.push_str(&num.to_string()),
                },
                Value::Null => render.push_str("null"),
                Value::Map(..) => return Err(MatcherError::InterpolatorRenderError {
                    template: self.template.to_owned(),
                    rule_name: self.rule_name.to_owned(),
                    cause: format!("Accessor [{:?}] returned a Map. Expected text, number, boolean or null.", accessor),
                }),
                Value::Array(..) => return Err(MatcherError::InterpolatorRenderError {
                    template: self.template.to_owned(),
                    rule_name: self.rule_name.to_owned(),
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
    use tornado_common_api::{Event, Payload};
    use tornado_common_parser::Parser;

    #[test]
    fn build_should_fail_if_not_valid_expression() {
        // Arrange
        let template = "<div>${test}</div>";

        // Act
        let interpolator = StringInterpolator::build(template, "rule", &Default::default());

        // Assert
        assert!(interpolator.is_err());
    }

    #[test]
    fn should_create_new_interpolator() {
        // Arrange
        let template = "<div><span>${event.payload.test}</sp${event.type}an><span>${_variables.test12}</span></${}div>";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();

        // Assert
        assert_eq!(3, interpolator.accessors.len());

        assert_eq!(&11, &interpolator.accessors[0].start);
        assert_eq!(&32, &interpolator.accessors[0].end);
        match &interpolator.accessors[0].accessor {
            Accessor::Payload { parser: Parser::Exp { keys } } => assert_eq!(1, keys.len()),
            _ => assert!(false),
        }

        assert_eq!(&36, &interpolator.accessors[1].start);
        assert_eq!(&49, &interpolator.accessors[1].end);
        match &interpolator.accessors[1].accessor {
            Accessor::Type => assert!(true),
            _ => assert!(false),
        }

        assert_eq!(&58, &interpolator.accessors[2].start);
        assert_eq!(&78, &interpolator.accessors[2].end);
        match &interpolator.accessors[2].accessor {
            Accessor::ExtractedVar { rule_name, key } => {
                assert_eq!("rule", rule_name);
                assert_eq!("test12", key);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_split_with_no_expressions_delimiters() {
        // Arrange
        let template = "constant string";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();

        // Assert
        assert_eq!(0, interpolator.accessors.len());
    }

    #[test]
    fn should_split_with_single_expression() {
        // Arrange
        let template = "${event.type}";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();

        // Assert
        assert_eq!(1, interpolator.accessors.len());

        assert_eq!(&0, &interpolator.accessors[0].start);
        assert_eq!(&13, &interpolator.accessors[0].end);
    }

    #[test]
    fn should_split_with_only_expressions() {
        // Arrange
        let template = "${event.type}${event.created_ms}${event.type}";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();

        // Assert
        assert_eq!(3, interpolator.accessors.len());

        assert_eq!(&0, &interpolator.accessors[0].start);
        assert_eq!(&13, &interpolator.accessors[0].end);

        assert_eq!(&13, &interpolator.accessors[1].start);
        assert_eq!(&32, &interpolator.accessors[1].end);

        assert_eq!(&32, &interpolator.accessors[2].start);
        assert_eq!(&45, &interpolator.accessors[2].end);
    }

    #[test]
    fn should_render_a_constant_string() {
        // Arrange
        let payload = Payload::new();

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ms: 1554130814854,
            payload,
        });

        let template = "constant string";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();
        let result = interpolator.render(&event, None);

        // Assert
        assert!(result.is_ok());
        let render = result.unwrap();

        assert_eq!("constant string", &render);
    }

    #[test]
    fn should_render_from_event() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ms: 1554130814854,
            payload,
        });

        let template = "type: ${event.type} - body: ${event.payload.body}";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();
        let result = interpolator.render(&event, None);

        // Assert
        assert!(result.is_ok());
        let render = result.unwrap();

        assert_eq!("type: event_type_value - body: body_value", &render);
    }

    #[test]
    fn should_render_from_extracted_vars() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ms: 1554130814854,
            payload,
        });
        let mut extracted_vars_inner = HashMap::new();
        extracted_vars_inner.insert("test1".to_owned(), Value::Text("var_test_1".to_owned()));
        extracted_vars_inner.insert("test2".to_owned(), Value::Text("var_test_2".to_owned()));

        let mut extracted_vars = HashMap::new();
        extracted_vars.insert("rule_for_test".to_owned(), extracted_vars_inner);

        let template = "1: ${_variables.test1} - 2: ${_variables.test2}";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule_for_test", &Default::default()).unwrap();
        let result = interpolator.render(&event, Some(&extracted_vars));

        // Assert
        assert!(result.is_ok());
        let render = result.unwrap();

        assert_eq!("1: var_test_1 - 2: var_test_2", &render);
    }

    #[test]
    fn should_render_numbers() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
        payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ms: 1554130814854,
            payload,
        });

        let template = "${event.created_ms}";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();
        let result = interpolator.render(&event, None);

        // Assert
        assert!(result.is_ok());
        let render = result.unwrap();

        assert_eq!("1554130814854", &render);
    }

    #[test]
    fn should_render_booleans() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("success".to_owned(), Value::Bool(true));
        payload.insert("fail".to_owned(), Value::Bool(false));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ms: 1554130814854,
            payload,
        });

        let template = "success: ${event.payload.success} - fail: ${event.payload.fail}";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();
        let result = interpolator.render(&event, None);

        // Assert
        assert!(result.is_ok());
        let render = result.unwrap();

        assert_eq!("success: true - fail: false", &render);
    }

    #[test]
    fn should_render_null() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("void".to_owned(), Value::Null);

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ms: 1554130814854,
            payload,
        });

        let template = " void:  ${event.payload.void} ";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();
        let result = interpolator.render(&event, None);

        // Assert
        assert!(result.is_ok());
        let render = result.unwrap();

        assert_eq!(" void:  null ", &render);
    }

    #[test]
    fn should_render_with_line_break() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("first".to_owned(), Value::Text("first line".to_owned()));
        payload.insert("second".to_owned(), Value::Text("second line".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ms: 1554130814854,
            payload,
        });

        let template = "${event.payload.first}\n${event.payload.second}";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();
        let result = interpolator.render(&event, None);

        // Assert
        assert!(result.is_ok());
        let render = result.unwrap();

        assert_eq!("first line\nsecond line", &render);
    }

    #[test]
    fn render_should_fail_if_no_value_is_present() {
        // Arrange
        let payload = Payload::new();

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ms: 1554130814854,
            payload,
        });

        let template = "${event.payload.second}";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();
        let result = interpolator.render(&event, None);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn render_should_fail_if_value_is_an_array() {
        // Arrange
        let body = vec![Value::Text("".to_owned())];

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Array(body));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ms: 1554130814854,
            payload,
        });

        let template = "${event.payload.body}";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();
        let result = interpolator.render(&event, None);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn render_should_fail_if_value_is_a_map() {
        // Arrange
        let mut body = HashMap::new();
        body.insert("".to_owned(), Value::Text("".to_owned()));

        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Map(body));

        let event = InternalEvent::new(Event {
            event_type: "event_type_value".to_owned(),
            created_ms: 1554130814854,
            payload,
        });

        let template = "${event.payload.body}";

        // Act
        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();
        let result = interpolator.render(&event, None);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn interpolator_demo() {
        // Arrange
        let mut payload = Payload::new();
        payload.insert("body".to_owned(), Value::Text("payload content".to_owned()));

        let event = InternalEvent::new(Event {
            event_type: "email".to_owned(),
            created_ms: 1554130814854,
            payload,
        });

        let template = r#"
            Received event with type: ${event.type}
            timestamp: ${event.created_ms}
            body content: ${event.payload.body}
         "#;

        // Act

        let interpolator =
            StringInterpolator::build(template, "rule", &Default::default()).unwrap();
        let result = interpolator.render(&event, None);

        // Assert
        assert!(result.is_ok());

        println!("---------------------------");
        println!("Event: \n{:?}", event);
        println!("Template: \n{}", template);
        println!("Rendered template: \n{}", result.unwrap());
        println!("---------------------------");
    }

    #[test]
    fn should_return_whether_the_interpolator_is_required() {
        let template = "<div><span>${event.payload.test}</sp${event.type}an><span>${_variables.test12}</span></${}div>";
        assert!(StringInterpolator::build(template, "", &Default::default())
            .unwrap()
            .is_interpolation_required());

        let template = " ${event.type}";
        assert!(StringInterpolator::build(template, "", &Default::default())
            .unwrap()
            .is_interpolation_required());

        let template = "${event.type} ";
        assert!(StringInterpolator::build(template, "", &Default::default())
            .unwrap()
            .is_interpolation_required());

        let template = "${event.type}";
        assert!(!StringInterpolator::build(template, "", &Default::default())
            .unwrap()
            .is_interpolation_required());

        let template = "${event.type}${event.type}";
        assert!(StringInterpolator::build(template, "", &Default::default())
            .unwrap()
            .is_interpolation_required());

        let template = "the type is: ${event.type}";
        assert!(StringInterpolator::build(template, "", &Default::default())
            .unwrap()
            .is_interpolation_required());

        let template = "constant text";
        assert!(!StringInterpolator::build(template, "", &Default::default())
            .unwrap()
            .is_interpolation_required());

        let template = "constant with empty expression ${}";
        assert!(!StringInterpolator::build(template, "", &Default::default())
            .unwrap()
            .is_interpolation_required());
    }
}
