use accessor::Accessor;
use config::{Extractor, ExtractorRegex};
use error::MatcherError;
use regex::{Match, Regex as RustRegex};
use tornado_common_api::Event;
use std::borrow::Cow;

pub struct MatcherExtractor {
    regex: RustRegex,
    group_match_idx: u16,
    target: Accessor,
}

impl MatcherExtractor {
    pub fn build(regex: &str, group_match_idx: u16, target: Accessor) -> Result<MatcherExtractor, MatcherError> {
        let regex = RustRegex::new(regex).map_err(|e| MatcherError::ExtractorBuildFailError {
            message: format!("Cannot parse regex [{}]", regex),
            cause: e.to_string(),
        })?;

        Ok(MatcherExtractor {
            target,
            group_match_idx,
            regex })
    }


    // To be tested:
    // - accessor returns SOME -> no regex matches
    // - accessor returns SOME -> regex matches but wrong group_match_idx
    // - accessor returns NONE
    pub fn extract(&self, event: &Event) -> Option<String> {
        let value = self.target.get(event)?;
        let captures = self.regex.captures(&value )?;
        let group_idx = self.group_match_idx;
        captures.get(group_idx as usize )
            .map(|matched| matched.as_str().to_owned() )
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use accessor::AccessorBuilder;
    use std::collections::HashMap;

    #[test]
    fn should_build_an_extractor() {
        let extractor = MatcherExtractor::build(
            "",
            0,
            AccessorBuilder::new().build("").unwrap()
        );
        assert!(extractor.is_ok());
    }

    #[test]
    fn build_should_fail_if_not_valid_regex() {
        let extractor = MatcherExtractor::build(
            "[",
            0,
            AccessorBuilder::new().build("").unwrap()
        );
        assert!(extractor.is_err());
    }

    #[test]
    fn should_match_and_return_group_at_zero() {
        let extractor = MatcherExtractor::build(
            r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?",
            0,
            AccessorBuilder::new().build("${event.type}").unwrap()
        ).unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!("http://stackoverflow.com/".to_owned(), extractor.extract(&event).unwrap());
    }

    #[test]
    fn should_match_and_return_group_at_one() {
        let extractor = MatcherExtractor::build(
            r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?",
            1,
            AccessorBuilder::new().build("${event.type}").unwrap()
        ).unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!("http".to_owned(), extractor.extract(&event).unwrap());
    }

    #[test]
    fn should_match_and_return_group_at_two() {
        let extractor = MatcherExtractor::build(
            r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?",
            2,
            AccessorBuilder::new().build("${event.type}").unwrap()
        ).unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!("stackoverflow.com".to_owned(), extractor.extract(&event).unwrap());
    }

    #[test]
    fn should_match_and_return_none_if_not_valid_group() {
        let extractor = MatcherExtractor::build(
            r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?",
            10000,
            AccessorBuilder::new().build("${event.type}").unwrap()
        ).unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert!(extractor.extract(&event).is_none());
    }

    #[test]
    fn should_match_and_return_none_if_not_value_from_event() {
        let extractor = MatcherExtractor::build(
            r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?",
            10000,
            AccessorBuilder::new().build("${event.payload.body}").unwrap()
        ).unwrap();

        let event = new_event("");

        assert!(extractor.extract(&event).is_none());
    }


    fn new_event(event_type: &str) -> Event {
        Event{
            payload: HashMap::new(),
            event_type: event_type.to_owned(),
            created_ts: 0
        }
    }
}