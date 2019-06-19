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
    static ref RE: Regex = Regex::new(r"(\$\{[^}]+})").unwrap();
}

/// A StringInterpolator allows the dynamic substitution of placeholders in a string
/// with values extracted from an incoming event.
/// E.g.:
/// ```rust
/// ```
pub struct StringInterpolator {
    template: String,
    rule_name: String,
    accessors: Vec<Accessor>,
}

impl StringInterpolator {
    /// Creates a new StringInterpolator
    pub fn build(
        template: &str,
        rule_name: &str,
        accessor_builder: &AccessorBuilder,
    ) -> Result<Self, MatcherError> {
        let accessors = StringInterpolator::split(template)
            .iter()
            .map(|part| accessor_builder.build(rule_name, part))
            .collect::<Result<Vec<_>, MatcherError>>()?;

        Ok(StringInterpolator {
            template: template.to_owned(),
            rule_name: rule_name.to_owned(),
            accessors,
        })
    }

    /// Returns whether the template used to create this StringInterpolator
    /// requires interpolation.
    /// This is true only if the template contains at least a static (e.g. constant text)
    /// and a dynamic part (e.g. placeholders to be resolved at runtime)
    pub fn is_interpolation_required(&self) -> bool {
        self.accessors.len() > 1
    }

    /// Render
    pub fn render(
        &self,
        event: &InternalEvent,
        extracted_vars: Option<&HashMap<String, Value>>,
    ) -> Result<Value, MatcherError> {
        let mut render = String::new();

        for accessor in &self.accessors {
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
        }

        Ok(Value::Text(render))
    }

    fn split(template: &str) -> Vec<&str> {
        let matches: Vec<(usize, usize)> =
            RE.find_iter(template).map(|m| (m.start(), m.end())).collect();

        let mut parts = vec![];

        // keeps the index of the previous argument end
        let mut prev_end = 0;

        // loop all matches
        for (start, end) in matches.iter() {
            // copy from previous argument end till current argument start
            if prev_end != *start {
                parts.push(&template[prev_end..*start])
            }

            // argument name with braces
            parts.push(&template[*start..*end]);

            prev_end = *end;
        }

        let template_len = template.len();

        // if last arg end index isn't the end of the string then copy
        // from last arg end till end of template
        if prev_end < template_len {
            parts.push(&template[prev_end..template_len])
        }

        parts
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tornado_common_api::{Event, Payload};

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
        assert_eq!(7, interpolator.accessors.len());

        match &interpolator.accessors[0] {
            Accessor::Constant { value } => assert_eq!("<div><span>", value),
            _ => assert!(false),
        }

        match &interpolator.accessors[1] {
            Accessor::Payload { keys } => assert_eq!(1, keys.len()),
            _ => assert!(false),
        }

        match &interpolator.accessors[2] {
            Accessor::Constant { value } => assert_eq!("</sp", value),
            _ => assert!(false),
        }

        match &interpolator.accessors[3] {
            Accessor::Type => assert!(true),
            _ => assert!(false),
        }

        match &interpolator.accessors[4] {
            Accessor::Constant { value } => assert_eq!("an><span>", value),
            _ => assert!(false),
        }

        match &interpolator.accessors[5] {
            Accessor::ExtractedVar { key } => assert_eq!("rule.test12", key),
            _ => assert!(false),
        }

        match &interpolator.accessors[6] {
            Accessor::Constant { value } => assert_eq!("</span></${}div>", value),
            _ => assert!(false),
        }
    }

    #[test]
    fn should_split_based_on_expressions_delimiters() {
        // Arrange
        let template = "<div><span>${event.payload.test}</sp${event.type}an><span>${_variables.test12}</span></${}div>";

        // Act
        let parts = StringInterpolator::split(template);

        // Assert
        assert_eq!(7, parts.len());
        assert_eq!("<div><span>", parts[0]);
        assert_eq!("${event.payload.test}", parts[1]);
        assert_eq!("</sp", parts[2]);
        assert_eq!("${event.type}", parts[3]);
        assert_eq!("an><span>", parts[4]);
        assert_eq!("${_variables.test12}", parts[5]);
        assert_eq!("</span></${}div>", parts[6]);
    }

    #[test]
    fn should_split_with_no_expressions_delimiters() {
        // Arrange
        let template = "constant string";

        // Act
        let parts = StringInterpolator::split(template);

        // Assert
        assert_eq!(1, parts.len());
        assert_eq!("constant string", parts[0]);
    }

    #[test]
    fn should_split_with_single_expression() {
        // Arrange
        let template = "${event.type}";

        // Act
        let parts = StringInterpolator::split(template);

        // Assert
        assert_eq!(1, parts.len());
        assert_eq!("${event.type}", parts[0]);
    }

    #[test]
    fn should_split_with_only_expressions() {
        // Arrange
        let template = "${event.type}${event.time_stamp}${event.type}";

        // Act
        let parts = StringInterpolator::split(template);

        // Assert
        println!("{:#?}", parts);
        assert_eq!(3, parts.len());
        assert_eq!("${event.type}", parts[0]);
        assert_eq!("${event.time_stamp}", parts[1]);
        assert_eq!("${event.type}", parts[2]);
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
        let mut extracted_vars = HashMap::new();
        extracted_vars
            .insert("rule_for_test.test1".to_owned(), Value::Text("var_test_1".to_owned()));
        extracted_vars
            .insert("rule_for_test.test2".to_owned(), Value::Text("var_test_2".to_owned()));

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
