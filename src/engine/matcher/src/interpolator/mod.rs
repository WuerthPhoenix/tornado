//
// For performance considerations check:
// - https://lise-henry.github.io/articles/optimising_strings.html
// - https://users.rust-lang.org/t/fast-string-concatenation/4425
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
}