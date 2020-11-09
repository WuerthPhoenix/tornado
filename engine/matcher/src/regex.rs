use crate::error::MatcherError;
use regex::Regex;
use std::ops::Deref;

/// A struct that allow high level operation on a basic Regex.
/// For example, this allow a regex to be used in PartialEq checks.
#[derive(Debug)]
pub struct RegexWrapper {
    regex_string: String,
    regex: Regex,
}

impl RegexWrapper {
    pub fn new<S: Into<String>>(regex_string: S) -> Result<Self, MatcherError> {
        let regex_string = regex_string.into();
        let regex =
            Regex::new(&regex_string).map_err(|e| MatcherError::ExtractorBuildFailError {
                message: format!("Cannot parse regex [{}]", regex_string),
                cause: e.to_string(),
            })?;
        Ok(Self { regex, regex_string })
    }

    pub fn regex(&self) -> &Regex {
        &self.regex
    }
}

impl Deref for RegexWrapper {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        self.regex()
    }
}

impl PartialEq for RegexWrapper {
    fn eq(&self, other: &Self) -> bool {
        other.regex_string.eq(&self.regex_string)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_compare_regexes() {
        assert_eq!(RegexWrapper::new(".*").unwrap(), RegexWrapper::new(".*").unwrap());
        assert_ne!(RegexWrapper::new(".*").unwrap(), RegexWrapper::new(".*[1-9]").unwrap());
    }
}
