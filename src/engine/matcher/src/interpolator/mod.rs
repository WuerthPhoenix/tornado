//
// For performance considerations see:
// - https://lise-henry.github.io/articles/optimising_strings.html
// - https://users.rust-lang.org/t/fast-string-concatenation/4425
// - https://github.com/hoodie/concatenation_benchmarks-rs
//

use lazy_static::*;
use regex::Regex;
use crate::accessor::{AccessorBuilder, Accessor};
use crate::error::MatcherError;

pub struct StringInterpolator {
    accessors: Vec<Accessor>}

impl StringInterpolator {
    pub fn build(template: &str, accessor_builder: &AccessorBuilder) -> Result<Self, MatcherError> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(\$\{[^}]+})").unwrap();
        }

        let mut accessors = vec![];

       // let regex: Regex = RE;

        let mut matches: Vec<(usize, usize)> = vec![];

        for capture in RE.captures_iter(template) {
            let text = &capture[1];
            println!("{}", text);
            accessors.push(accessor_builder.build("", text)?);
        }

        Ok(StringInterpolator {
            accessors
        })
    }

    fn split(template: &str) -> Result<Vec<&str>, MatcherError> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(\$\{[^}]+})").unwrap();
        }


        let matches: Vec<(usize, usize)> = RE.find_iter(template)
            .map(|m| (m.start(), m.end()))
            .collect();

        // get index of first arg match or return a copy of the template if no args matched
        let first = match matches.first() {
            Some((start, _)) => *start,
            _ => return Ok(vec![template]),
        };

        let mut parts = vec![];

        // copy from template start to first arg
        if first > 0 {
            parts.push(&template[0..first])
        }

        // keeps the index of the previous argument end
        let mut prev_end: Option<usize> = None;

        // loop all matches
        for (start, end) in matches.iter() {
            // copy from previous argument end till current argument start
            if let Some(last_end) = prev_end {
                if last_end != *start {
                    parts.push(&template[last_end..*start])
                }
            }

            // argument name with braces
            parts.push(&template[*start..*end]);

            prev_end = Some(*end);
        }

        let template_len = template.len();

        // if last arg end index isn't the end of the string then copy
        // from last arg end till end of template
        if let Some(last_pos) = prev_end {
            if last_pos < template_len {
                parts.push(&template[last_pos..template_len])
            }
        }

        Ok(parts)
    }

}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn build_should_fail_if_not_valid_expression() {
        // Arrange
        let template = "<div>${test}</div>";

        // Act
        let interpolator = StringInterpolator::build(template, &Default::default());

        // Assert
        assert!(interpolator.is_err());
    }

    #[test]
    fn should_create_new_interpolator() {
        // Arrange
        let template = "<div><span>${event.payload.test}</sp${event.type}an><span>${_variables.test12}</span></${}div>";

        // Act
        let interpolator = StringInterpolator::build(template, &Default::default()).unwrap();

        // Assert
        assert_eq!(3, interpolator.accessors.len());

        match &interpolator.accessors[0] {
            Accessor::Constant {value} => assert_eq!("<div><span>", value),
            _ => assert!(false)
        }

        match &interpolator.accessors[1] {
            Accessor::Payload {keys} => assert_eq!(1, keys.len()),
            _ => assert!(false)
        }

        match &interpolator.accessors[2] {
            Accessor::Constant {value} => assert_eq!("</sp", value),
            _ => assert!(false)
        }

        match &interpolator.accessors[3] {
            Accessor::Type  => assert!(true),
            _ => assert!(false)
        }

        match &interpolator.accessors[4] {
            Accessor::Constant {value} => assert_eq!("an><span>", value),
            _ => assert!(false)
        }

        match &interpolator.accessors[5] {
            Accessor::ExtractedVar { key }  => assert_eq!("test12", key),
            _ => assert!(false)
        }

        match &interpolator.accessors[6] {
            Accessor::Constant {value} => assert_eq!("</span></${}div>", value),
            _ => assert!(false)
        }
    }

    #[test]
    fn should_split_based_on_expressions_delimiters() {
        // Arrange
        let template = "<div><span>${event.payload.test}</sp${event.type}an><span>${_variables.test12}</span></${}div>";

        // Act
        let parts = StringInterpolator::split(template).unwrap();

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
        let parts = StringInterpolator::split(template).unwrap();

        // Assert
        assert_eq!(1, parts.len());
        assert_eq!("constant string", parts[0]);
    }

    #[test]
    fn should_split_with_single_expression() {
        // Arrange
        let template = "${event.type}";

        // Act
        let parts = StringInterpolator::split(template).unwrap();

        // Assert
        assert_eq!(1, parts.len());
        assert_eq!("${event.type}", parts[0]);
    }

    #[test]
    fn should_split_with_only_expressions() {
        // Arrange
        let template = "${event.type}${event.time_stamp}${event.type}";

        // Act
        let parts = StringInterpolator::split(template).unwrap();

        // Assert
        println!("{:#?}", parts);
        assert_eq!(3, parts.len());
        assert_eq!("${event.type}", parts[0]);
        assert_eq!("${event.time_stamp}", parts[1]);
        assert_eq!("${event.type}", parts[2]);
    }
}