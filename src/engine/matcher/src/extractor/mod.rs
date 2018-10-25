use accessor::{Accessor, AccessorBuilder};
use config::Extractor;
use error::MatcherError;
use model::ProcessedEvent;
use regex::Regex as RustRegex;
use std::collections::HashMap;

/// MatcherExtractor instance builder.
#[derive(Default)]
pub struct MatcherExtractorBuilder {
    accessor: AccessorBuilder,
}

impl MatcherExtractorBuilder {
    /// Returns a new MatcherExtractorBuilder instance
    pub fn new() -> MatcherExtractorBuilder {
        MatcherExtractorBuilder {
            accessor: AccessorBuilder::new(),
        }
    }

    /// Returns a specific MatcherExtractor instance based on the rule extractor configuration.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    ///    extern crate tornado_common_api;
    ///    extern crate tornado_engine_matcher;
    ///
    ///    use tornado_common_api::Event;
    ///    use tornado_engine_matcher::extractor::MatcherExtractorBuilder;
    ///    use tornado_engine_matcher::config::{Extractor, ExtractorRegex};
    ///    use tornado_engine_matcher::model::ProcessedEvent;
    ///    use std::collections::HashMap;
    ///
    ///    let mut extractor_config = HashMap::new();
    ///
    ///    extractor_config.insert(
    ///        String::from("extracted_temp"),
    ///        Extractor {
    ///            from: String::from("${event.type}"),
    ///            regex: ExtractorRegex {
    ///                regex: String::from(r"[0-9]+"),
    ///                group_match_idx: 0,
    ///            },
    ///        },
    ///    );
    ///
    ///    // The matcher_extractor contains the logic to create the "extracted_temp" variable from the ${event.type}.
    ///    // The value of the "extracted_temp" variable is obtained applying the regular expression "[0-9]+" to the event.type.
    ///    let matcher_extractor = MatcherExtractorBuilder::new().build("rule_name", &extractor_config).unwrap();
    ///
    ///    let event = ProcessedEvent::new(Event {
    ///        payload: HashMap::new(),
    ///        event_type: "temp=44'C".to_owned(),
    ///        created_ts: 0,
    ///    });
    ///
    ///    assert_eq!(
    ///        String::from("44"),
    ///        matcher_extractor.extract("extracted_temp", &event).unwrap()
    ///    );
    /// ```
    pub fn build(
        &self,
        rule_name: &str,
        config: &HashMap<String, Extractor>,
    ) -> Result<MatcherExtractor, MatcherError> {
        let mut matcher_extractor = MatcherExtractor {
            extractors: HashMap::new(),
        };
        for (k, v) in config.iter() {
            matcher_extractor.extractors.insert(
                k.to_owned(),
                VariableExtractor::build(
                    &v.regex.regex,
                    v.regex.group_match_idx,
                    self.accessor.build(rule_name, &v.from)?,
                )?,
            );
        }

        info!(
            "MatcherExtractorBuilder - build: built extractor [{:?}] for input value [{:?}]",
            &matcher_extractor, config
        );

        Ok(matcher_extractor)
    }
}

#[derive(Debug)]
pub struct MatcherExtractor {
    extractors: HashMap<String, VariableExtractor>,
}

impl MatcherExtractor {
    /// Returns the value of the variable with name 'key' generated from the provided Event
    pub fn extract(&self, key: &str, event: &ProcessedEvent) -> Result<String, MatcherError> {
        let extracted = self
            .extractors
            .get(key)
            .and_then(|extractor| extractor.extract(event));
        self.check_extracted(key, extracted)
    }

    /// Returns the value of all extracted variables defined in the rule and generated from the provided Event.
    /// Returns an Error if not all variables can be correctly extracted.
    pub fn extract_all<'o>(
        &'o self,
        event: &ProcessedEvent,
    ) -> Result<HashMap<&'o str, String>, MatcherError> {
        let mut vars = HashMap::new();
        for (key, extractor) in &self.extractors {
            let value = self.check_extracted(key, extractor.extract(event))?;
            vars.insert(key.as_str(), value);
        }
        Ok(vars)
    }

    fn check_extracted(
        &self,
        key: &str,
        extracted: Option<String>,
    ) -> Result<String, MatcherError> {
        match extracted {
            Some(value) => Ok(value),
            None => Err(MatcherError::MissingExtractedVariableError {
                variable_name: key.to_owned(),
            }),
        }
    }
}

#[derive(Debug)]
struct VariableExtractor {
    regex: RustRegex,
    group_match_idx: u16,
    target: Accessor,
}

impl VariableExtractor {
    pub fn build(
        regex: &str,
        group_match_idx: u16,
        target: Accessor,
    ) -> Result<VariableExtractor, MatcherError> {
        let regex = RustRegex::new(regex).map_err(|e| MatcherError::ExtractorBuildFailError {
            message: format!("Cannot parse regex [{}]", regex),
            cause: e.to_string(),
        })?;

        Ok(VariableExtractor {
            target,
            group_match_idx,
            regex,
        })
    }

    pub fn extract(&self, event: &ProcessedEvent) -> Option<String> {
        let value = self.target.get(event)?;
        let captures = self.regex.captures(&value)?;
        let group_idx = self.group_match_idx;
        captures
            .get(group_idx as usize)
            .map(|matched| matched.as_str().to_owned())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use accessor::AccessorBuilder;
    use config::ExtractorRegex;
    use std::collections::HashMap;
    use tornado_common_api::Event;

    #[test]
    fn should_build_an_extractor() {
        let extractor =
            VariableExtractor::build("", 0, AccessorBuilder::new().build("", "").unwrap());
        assert!(extractor.is_ok());
    }

    #[test]
    fn build_should_fail_if_not_valid_regex() {
        let extractor =
            VariableExtractor::build("[", 0, AccessorBuilder::new().build("", "").unwrap());
        assert!(extractor.is_err());
    }

    #[test]
    fn should_match_and_return_group_at_zero() {
        let extractor = VariableExtractor::build(
            r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?",
            0,
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
        ).unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!(
            "http://stackoverflow.com/".to_owned(),
            extractor.extract(&event).unwrap()
        );
    }

    #[test]
    fn should_match_and_return_group_at_one() {
        let extractor = VariableExtractor::build(
            r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?",
            1,
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
        ).unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!("http".to_owned(), extractor.extract(&event).unwrap());
    }

    #[test]
    fn should_match_and_return_group_at_two() {
        let extractor = VariableExtractor::build(
            r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?",
            2,
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
        ).unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!(
            "stackoverflow.com".to_owned(),
            extractor.extract(&event).unwrap()
        );
    }

    #[test]
    fn should_match_and_return_none_if_not_valid_group() {
        let extractor = VariableExtractor::build(
            r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?",
            10000,
            AccessorBuilder::new().build("", "${event.type}").unwrap(),
        ).unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert!(extractor.extract(&event).is_none());
    }

    #[test]
    fn should_match_and_return_none_if_not_value_from_event() {
        let extractor = VariableExtractor::build(
            r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?",
            10000,
            AccessorBuilder::new()
                .build("", "${event.payload.body}")
                .unwrap(),
        ).unwrap();

        let event = new_event("");

        assert!(extractor.extract(&event).is_none());
    }

    #[test]
    fn should_use_variable_extractor_based_on_variable_name() {
        let mut from_config = HashMap::new();

        from_config.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: 0,
                },
            },
        );

        from_config.insert(
            String::from("extracted_text"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[a-z]+"),
                    group_match_idx: 0,
                },
            },
        );

        let extractor = MatcherExtractorBuilder::new()
            .build("", &from_config)
            .unwrap();

        let event = new_event("temp=44'C");

        assert_eq!(
            String::from("44"),
            extractor.extract("extracted_temp", &event).unwrap()
        );
        assert_eq!(
            String::from("temp"),
            extractor.extract("extracted_text", &event).unwrap()
        );
    }

    #[test]
    fn should_return_none_if_unknown_variable_name() {
        let mut from_config = HashMap::new();

        from_config.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: 0,
                },
            },
        );

        let extractor = MatcherExtractorBuilder::new()
            .build("", &from_config)
            .unwrap();

        let event = new_event("temp=44'C");

        assert!(extractor.extract("extracted_text", &event).is_err());
    }

    #[test]
    fn should_extract_all_variables_and_return_true() {
        let mut from_config = HashMap::new();

        from_config.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: 0,
                },
            },
        );

        from_config.insert(
            String::from("extracted_text"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[a-z]+"),
                    group_match_idx: 0,
                },
            },
        );

        let extractor = MatcherExtractorBuilder::new()
            .build("", &from_config)
            .unwrap();

        let event = new_event("temp=44'C");

        let vars = extractor.extract_all(&event).unwrap();

        assert_eq!(2, vars.len());
        assert_eq!(&String::from("44"), vars.get("extracted_temp").unwrap());
        assert_eq!(&String::from("temp"), vars.get("extracted_text").unwrap());
    }

    #[test]
    fn should_extract_all_variables_and_return_false_is_not_all_resolved() {
        let mut from_config = HashMap::new();

        from_config.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: 0,
                },
            },
        );

        from_config.insert(
            String::from("extracted_none"),
            Extractor {
                from: String::from("${event.payload.nothing}"),
                regex: ExtractorRegex {
                    regex: String::from(r"[a-z]+"),
                    group_match_idx: 0,
                },
            },
        );

        let extractor = MatcherExtractorBuilder::new()
            .build("", &from_config)
            .unwrap();

        let event = new_event("temp=44'C");

        assert!(extractor.extract_all(&event).is_err());
    }

    fn new_event(event_type: &str) -> ProcessedEvent {
        ProcessedEvent::new(Event {
            payload: HashMap::new(),
            event_type: event_type.to_owned(),
            created_ts: 0,
        })
    }
}
