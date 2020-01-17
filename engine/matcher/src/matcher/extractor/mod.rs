//! The extractor module contains the logic to generate variables based on the
//! Rule configuration.
//!
//! An *Extractor* is linked to the "WITH" clause of a Rule and determines the value
//! of dynamically generated variables.

use crate::accessor::{Accessor, AccessorBuilder};
use crate::config::rule::{Extractor, ExtractorRegex};
use crate::error::MatcherError;
use crate::model::InternalEvent;
use log::*;
use regex::{Captures, Regex as RustRegex};
use std::collections::HashMap;
use tornado_common_api::Value;

/// The MatcherExtractor instance builder.
#[derive(Default)]
pub struct MatcherExtractorBuilder {
    accessor: AccessorBuilder,
}

impl MatcherExtractorBuilder {
    /// Returns a new MatcherExtractorBuilder instance.
    pub fn new() -> MatcherExtractorBuilder {
        MatcherExtractorBuilder { accessor: AccessorBuilder::new() }
    }

    /// Returns a specific MatcherExtractor instance based on the matcher.extractor rule configuration.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    ///    use tornado_common_api::{Event, Value};
    ///    use tornado_engine_matcher::matcher::extractor::MatcherExtractorBuilder;
    ///    use tornado_engine_matcher::config::rule::{Extractor, ExtractorRegex};
    ///    use tornado_engine_matcher::model::InternalEvent;
    ///    use std::collections::HashMap;
    ///
    ///    let mut extractor_config = HashMap::new();
    ///
    ///    extractor_config.insert(
    ///        String::from("extracted_temp"),
    ///        Extractor {
    ///            from: String::from("${event.type}"),
    ///            regex: ExtractorRegex::Regex {
    ///                regex: String::from(r"[0-9]+"),
    ///                group_match_idx: Some(0),
    ///                all_matches: None,
    ///            },
    ///        },
    ///    );
    ///
    ///    // The matcher_extractor contains the logic to create the "extracted_temp" variable from the ${event.type}.
    ///    // The value of the "extracted_temp" variable is obtained by applying the regular expression "[0-9]+" to
    ///    // the event.type.
    ///    let matcher_extractor = MatcherExtractorBuilder::new().build("rule_name", &extractor_config).unwrap();
    ///
    ///    let event: InternalEvent = Event::new("temp=44'C").into();
    ///    let mut extracted_vars = Value::Map(HashMap::new());
    ///
    ///    let result = matcher_extractor.process_all(&event, &mut extracted_vars);
    ///
    ///    assert!(result.is_ok());
    ///    assert_eq!(1, extracted_vars.get_map().unwrap().len());
    ///    assert_eq!(
    ///        "44",
    ///        extracted_vars.get_from_map("rule_name").unwrap().get_from_map("extracted_temp").unwrap()
    ///    );
    /// ```
    pub fn build(
        &self,
        rule_name: &str,
        config: &HashMap<String, Extractor>,
    ) -> Result<MatcherExtractor, MatcherError> {
        let mut matcher_extractor =
            MatcherExtractor { rule_name: rule_name.to_owned(), extractors: HashMap::new() };
        for (key, extractor) in config.iter() {
            matcher_extractor.extractors.insert(
                key.to_owned(),
                ValueExtractor::build(rule_name, key, extractor, &self.accessor)?,
            );
        }

        trace!(
            "MatcherExtractorBuilder - build: built matcher.extractor [{:?}] for input value [{:?}]",
            &matcher_extractor, config
        );

        Ok(matcher_extractor)
    }
}

#[derive(Debug)]
pub struct MatcherExtractor {
    rule_name: String,
    extractors: HashMap<String, ValueExtractor>,
}

impl MatcherExtractor {
    /*
    /// Returns the value of the variable named 'key' generated from the provided Event.
    fn extract(&self, key: &str, event: &InternalEvent, extracted_vars: Option<&Value>) -> Result<String, MatcherError> {
        let extracted = self.extractors.get(key).and_then(|extractor| extractor.extract(event, extracted_vars));
        self.check_extracted(key, extracted)
    }
    */

    /// Fills the Event with the extracted variables defined in the rule and generated from the Event itself.
    /// Returns an Error if not all variables can be correctly extracted.
    /// The variable 'key' in the event.extracted_vars map has the form:
    /// rule_name.extracted_var_name
    pub fn process_all(
        &self,
        event: &InternalEvent,
        extracted_vars: &mut Value,
    ) -> Result<(), MatcherError> {
	    if !self.extractors.is_empty() {
	        let mut vars = HashMap::new();
	        for (key, extractor) in &self.extractors {
	            let value =
	                self.check_extracted(key, extractor.extract(event, Some(extracted_vars)))?;
	            vars.insert(extractor.key.to_string(), value);
	        }

            if let Some(map) = extracted_vars.get_map_mut() {
                map.insert(self.rule_name.to_string(), Value::Map(vars));
            } else {
                return Err(MatcherError::InternalSystemError {
                    message: "MatcherExtractor - process_all - expected a Value::Map".to_owned(),
                });
            }
        }
        Ok(())
    }

    fn check_extracted(&self, key: &str, extracted: Option<Value>) -> Result<Value, MatcherError> {
        match extracted {
            Some(value) => Ok(value),
            None => {
                Err(MatcherError::MissingExtractedVariableError { variable_name: key.to_owned() })
            }
        }
    }
}

#[derive(Debug)]
struct ValueExtractor {
    pub key: String,
    pub regex_extractor: RegexValueExtractor,
}

impl ValueExtractor {
    pub fn build(
        rule_name: &str,
        key: &str,
        extractor: &Extractor,
        accessor: &AccessorBuilder,
    ) -> Result<ValueExtractor, MatcherError> {
        Ok(Self {
            key: key.to_owned(),
            regex_extractor: RegexValueExtractor::build(rule_name, extractor, accessor)?,
        })
    }

    pub fn extract(&self, event: &InternalEvent, extracted_vars: Option<&Value>) -> Option<Value> {
        self.regex_extractor.extract(event, extracted_vars)
    }
}

#[derive(Debug)]
enum RegexValueExtractor {
    SingleMatchSingleGroup { regex: RustRegex, group_match_idx: usize, target: Accessor },
    AllMatchesSingleGroup { regex: RustRegex, group_match_idx: usize, target: Accessor },
    SingleMatchAllGroups { regex: RustRegex, target: Accessor },
    AllMatchesAllGroups { regex: RustRegex, target: Accessor },
    SingleMatchNamedGroups { regex: RustRegex, target: Accessor },
    AllMatchesNamedGroups { regex: RustRegex, target: Accessor },
}

impl RegexValueExtractor {
    pub fn build(
        rule_name: &str,
        extractor: &Extractor,
        accessor: &AccessorBuilder,
    ) -> Result<RegexValueExtractor, MatcherError> {
        let target = accessor.build(rule_name, &extractor.from)?;

        match &extractor.regex {
            ExtractorRegex::Regex { regex, group_match_idx, all_matches } => {
                let rust_regex =
                    RustRegex::new(regex).map_err(|e| MatcherError::ExtractorBuildFailError {
                        message: format!("Cannot parse regex [{}]", regex),
                        cause: e.to_string(),
                    })?;

                let all_matches = all_matches.unwrap_or(false);

                if has_named_groups(&rust_regex) {
                    warn!(
                        "The regex [{}] has named groups but the extractor is index based.",
                        regex
                    );
                }

                match group_match_idx {
                    Some(group_match_idx) => {
                        if all_matches {
                            Ok(RegexValueExtractor::AllMatchesSingleGroup {
                                target,
                                group_match_idx: *group_match_idx,
                                regex: rust_regex,
                            })
                        } else {
                            Ok(RegexValueExtractor::SingleMatchSingleGroup {
                                target,
                                group_match_idx: *group_match_idx,
                                regex: rust_regex,
                            })
                        }
                    }
                    None => {
                        if all_matches {
                            Ok(RegexValueExtractor::AllMatchesAllGroups {
                                target,
                                regex: rust_regex,
                            })
                        } else {
                            Ok(RegexValueExtractor::SingleMatchAllGroups {
                                target,
                                regex: rust_regex,
                            })
                        }
                    }
                }
            }
            ExtractorRegex::RegexNamedGroups { regex, all_matches } => {
                let rust_regex =
                    RustRegex::new(regex).map_err(|e| MatcherError::ExtractorBuildFailError {
                        message: format!("Cannot parse regex [{}]", regex),
                        cause: e.to_string(),
                    })?;

                if !has_named_groups(&rust_regex) {
                    return Err(MatcherError::ConfigurationError {
                        message: format!(
                            "The regex [{}] has no named groups but it is used in named_match.",
                            regex
                        ),
                    });
                }

                if all_matches.unwrap_or(false) {
                    Ok(RegexValueExtractor::AllMatchesNamedGroups { regex: rust_regex, target })
                } else {
                    Ok(RegexValueExtractor::SingleMatchNamedGroups { regex: rust_regex, target })
                }
            }
        }
    }

    pub fn extract(&self, event: &InternalEvent, extracted_vars: Option<&Value>) -> Option<Value> {
        match self {
            // Note: the non-'multi' implementations could be avoided as they are a particular case of the 'multi' ones;
            // however, we can use an optimized logic if we know beforehand that only the first capture is required
            RegexValueExtractor::SingleMatchSingleGroup { regex, group_match_idx, target } => {
                let cow_value = target.get(event, extracted_vars)?;
                let text = cow_value.get_text()?;
                let captures = regex.captures(text)?;
                captures
                    .get(*group_match_idx)
                    .map(|matched| Value::Text(matched.as_str().to_owned()))
            }
            RegexValueExtractor::AllMatchesSingleGroup { regex, group_match_idx, target } => {
                let cow_value = target.get(event, extracted_vars)?;
                let text = cow_value.get_text()?;

                let mut result = vec![];
                for captures in regex.captures_iter(text) {
                    if let Some(value) = captures
                        .get(*group_match_idx)
                        .map(|matched| Value::Text(matched.as_str().to_owned()))
                    {
                        result.push(value);
                    } else {
                        return None;
                    }
                }
                if !result.is_empty() {
                    Some(Value::Array(result))
                } else {
                    None
                }
            }
            RegexValueExtractor::SingleMatchAllGroups { regex, target } => {
                let cow_value = target.get(event, extracted_vars)?;
                let text = cow_value.get_text()?;
                let captures = regex.captures(text)?;

                if let Some(groups) = get_indexed_groups(&captures) {
                    Some(Value::Array(groups))
                } else {
                    None
                }
            }
            RegexValueExtractor::AllMatchesAllGroups { regex, target } => {
                let cow_value = target.get(event, extracted_vars)?;
                let text = cow_value.get_text()?;

                let mut result = vec![];
                for captures in regex.captures_iter(text) {
                    if let Some(groups) = get_indexed_groups(&captures) {
                        result.push(Value::Array(groups));
                    } else {
                        return None;
                    }
                }
                if !result.is_empty() {
                    Some(Value::Array(result))
                } else {
                    None
                }
            }
            RegexValueExtractor::SingleMatchNamedGroups { regex, target } => {
                let cow_value = target.get(event, extracted_vars)?;
                let text = cow_value.get_text()?;

                if let Some(captures) = regex.captures(text) {
                    if let Some(groups) = get_named_groups(&captures, regex) {
                        return Some(Value::Map(groups));
                    } else {
                        return None;
                    }
                };
                None
            }
            RegexValueExtractor::AllMatchesNamedGroups { regex, target } => {
                let cow_value = target.get(event, extracted_vars)?;
                let text = cow_value.get_text()?;
                let mut result = vec![];
                for captures in regex.captures_iter(text) {
                    if let Some(groups) = get_named_groups(&captures, regex) {
                        result.push(Value::Map(groups));
                    } else {
                        return None;
                    }
                }
                if !result.is_empty() {
                    Some(Value::Array(result))
                } else {
                    None
                }
            }
        }
    }
}

fn get_named_groups(captures: &Captures, regex: &RustRegex) -> Option<HashMap<String, Value>> {
    let mut groups = HashMap::new();
    for name in regex.capture_names() {
        if let Some(name) = name {
            if let Some(matched) = captures.name(name) {
                groups.insert(name.to_owned(), Value::Text(matched.as_str().to_owned()));
            } else {
                return None;
            }
        }
    }
    Some(groups)
}

fn get_indexed_groups(captures: &Captures) -> Option<Vec<Value>> {
    let mut groups = vec![];
    for capture in captures.iter() {
        if let Some(matched) = capture {
            groups.push(Value::Text(matched.as_str().to_owned()))
        } else {
            return None;
        }
    }
    Some(groups)
}

fn has_named_groups(regex: &RustRegex) -> bool {
    for name in regex.capture_names() {
        if name.is_some() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::accessor::AccessorBuilder;
    use crate::config::rule::ExtractorRegex;
    use maplit::*;
    use std::collections::HashMap;
    use tornado_common_api::Event;

    #[test]
    fn should_build_an_extractor() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: "".to_string(),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        );
        assert!(extractor.is_ok());
    }

    #[test]
    fn build_should_fail_if_not_valid_regex() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: "[".to_string(),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        );
        assert!(extractor.is_err());
    }

    #[test]
    fn should_match_and_return_group_at_zero() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!(
            Value::Text("http://stackoverflow.com/".to_owned()),
            extractor.extract(&event, None).unwrap()
        );
    }

    #[test]
    fn should_match_and_return_group_at_one() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(1),
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!(Value::Text("http".to_owned()), extractor.extract(&event, None).unwrap());
    }

    #[test]
    fn should_match_and_return_group_at_one_multi() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(1),
                    all_matches: Some(true),
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/\nftp://test.com");

        assert_eq!(
            Value::Array(vec![Value::Text("http".to_owned()), Value::Text("ftp".to_owned()),]),
            extractor.extract(&event, None).unwrap()
        );
    }

    #[test]
    fn should_match_and_return_group_at_two() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(2),
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!(
            Value::Text("stackoverflow.com".to_owned()),
            extractor.extract(&event, None).unwrap()
        );
    }

    #[test]
    fn should_match_and_return_none_if_not_valid_group() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(10000),
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert!(extractor.extract(&event, None).is_none());
    }

    #[test]
    fn should_match_and_return_none_if_not_valid_group_multi() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(10000),
                    all_matches: Some(true),
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert!(extractor.extract(&event, None).is_none());
    }

    #[test]
    fn should_match_and_return_none_if_not_value_from_event() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.payload.body}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(1),
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("");

        assert!(extractor.extract(&event, None).is_none());
    }

    #[test]
    fn should_extract_all_variables_and_return_true() {
        let mut from_config = HashMap::new();

        from_config.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
            },
        );

        from_config.insert(
            String::from("extracted_text"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[a-z]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
            },
        );

        let extractor = MatcherExtractorBuilder::new().build("rule", &from_config).unwrap();

        let event = new_event("temp=44'C");
        let mut extracted_vars = Value::Map(HashMap::new());

        extractor.process_all(&event, &mut extracted_vars).unwrap();

        assert_eq!(1, extracted_vars.get_map().unwrap().len());
        assert_eq!(2, extracted_vars.get_from_map("rule").unwrap().get_map().unwrap().len());
        assert_eq!(
            "44",
            extracted_vars.get_from_map("rule").unwrap().get_from_map("extracted_temp").unwrap()
        );
        assert_eq!(
            "temp",
            extracted_vars.get_from_map("rule").unwrap().get_from_map("extracted_text").unwrap()
        );
    }

    #[test]
    fn should_extract_all_variables_and_return_false_is_not_all_resolved() {
        let mut from_config = HashMap::new();

        from_config.insert(
            String::from("extracted_temp"),
            Extractor {
                from: String::from("${event.type}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
            },
        );

        from_config.insert(
            String::from("extracted_none"),
            Extractor {
                from: String::from("${event.payload.nothing}"),
                regex: ExtractorRegex::Regex {
                    regex: String::from(r"[a-z]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
            },
        );

        let extractor = MatcherExtractorBuilder::new().build("", &from_config).unwrap();

        let mut event = new_event("temp=44'C");
        let mut extracted_vars = Value::Map(HashMap::new());

        assert!(extractor.process_all(&mut event, &mut extracted_vars).is_err());
    }

    #[test]
    fn should_return_all_matching_groups_if_no_idx() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: None,
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!(
            Value::Array(vec![
                Value::Text("http://stackoverflow.com/".to_owned()),
                Value::Text("http".to_owned()),
                Value::Text("stackoverflow.com".to_owned()),
                Value::Text("/".to_owned()),
            ]),
            extractor.extract(&event, None).unwrap()
        );
    }

    #[test]
    fn should_return_all_matching_groups_multi_if_no_idx() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^.\n]+).([^.\n]*)?".to_string(),
                    group_match_idx: None,
                    all_matches: Some(true),
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com\nftp://test.org");

        assert_eq!(
            extractor.extract(&event, None),
            Some(Value::Array(vec![
                Value::Array(vec![
                    Value::Text("http://stackoverflow.com".to_owned()),
                    Value::Text("http".to_owned()),
                    Value::Text("stackoverflow".to_owned()),
                    Value::Text("com".to_owned()),
                ]),
                Value::Array(vec![
                    Value::Text("ftp://test.org".to_owned()),
                    Value::Text("ftp".to_owned()),
                    Value::Text("test".to_owned()),
                    Value::Text("org".to_owned()),
                ])
            ])),
        );
    }

    #[test]
    fn should_fail_if_not_all_matching_groups_are_found() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^.\n]+).(/[^.\n]*)?".to_string(),
                    group_match_idx: None,
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow/");

        assert_eq!(None, extractor.extract(&event, None));
    }

    #[test]
    fn should_fail_if_a_matching_group_is_not_found_even_if_regex_matches() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"ab(c)*d".to_string(),
                    group_match_idx: None,
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("abd");

        assert_eq!(None, extractor.extract(&event, None));
    }

    #[test]
    fn should_fail_if_not_all_matching_groups_are_found_with_multi() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(https?|ftp)://([^.\n]+).(/[^.\n]*)?".to_string(),
                    group_match_idx: None,
                    all_matches: Some(true),
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow/");

        assert_eq!(None, extractor.extract(&event, None));
    }

    #[test]
    fn should_return_array_of_values_even_with_named_groups() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::Regex {
                    regex: r"(?P<PROTOCOL>https?|ftp)://(?P<NAME>[^.\n]+).(?P<EXTENSION>[^.\n]*)?"
                        .to_string(),
                    group_match_idx: None,
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com");

        assert_eq!(
            Value::Array(vec![
                Value::Text("http://stackoverflow.com".to_owned()),
                Value::Text("http".to_owned()),
                Value::Text("stackoverflow".to_owned()),
                Value::Text("com".to_owned()),
            ]),
            extractor.extract(&event, None).unwrap()
        );
    }

    #[test]
    fn should_return_map_with_named_groups() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::RegexNamedGroups {
                    regex: r"(?P<PROTOCOL>https?|ftp)://(?P<NAME>[^.\n]+).(?P<EXTENSION>[^.\n]*)?"
                        .to_string(),
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com");

        assert_eq!(
            extractor.extract(&event, None).unwrap(),
            Value::Map(hashmap![
                "PROTOCOL".to_string() => Value::Text("http".to_owned()),
                "NAME".to_string() => Value::Text("stackoverflow".to_owned()),
                "EXTENSION".to_string() => Value::Text("com".to_owned()),
            ]),
        );
    }

    #[test]
    fn should_return_map_with_named_groups_ignoring_unnamed_groups() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::RegexNamedGroups {
                    regex: r"(?P<PROTOCOL>https?|ftp)://([^.\n]+).(?P<EXTENSION>[^.\n]*)?"
                        .to_string(),
                    all_matches: Some(false),
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com");

        assert_eq!(
            extractor.extract(&event, None).unwrap(),
            Value::Map(hashmap![
                "PROTOCOL".to_string() => Value::Text("http".to_owned()),
                "EXTENSION".to_string() => Value::Text("com".to_owned()),
            ]),
        );
    }

    #[test]
    fn should_return_error_if_regex_with_named_groups_does_not_match() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::RegexNamedGroups {
                    regex: r"(?P<PROTOCOL>https?|ftp)://(?P<NAME>[^.\n]+)?".to_string(),
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("123");

        assert_eq!(extractor.extract(&event, None), None,);
    }

    #[test]
    fn should_return_multi_map_with_named_groups() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::RegexNamedGroups {
                    regex: r"(?P<PROTOCOL>https?|ftp)://(?P<NAME>[^.\n]+).(?P<EXTENSION>[^.\n]*)?"
                        .to_string(),
                    all_matches: Some(true),
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com\nftp://test.org");

        assert_eq!(
            extractor.extract(&event, None),
            Some(Value::Array(vec![
                Value::Map(hashmap![
                    "PROTOCOL".to_string() => Value::Text("http".to_owned()),
                    "NAME".to_string() => Value::Text("stackoverflow".to_owned()),
                    "EXTENSION".to_string() => Value::Text("com".to_owned()),
                ]),
                Value::Map(hashmap![
                    "PROTOCOL".to_string() => Value::Text("ftp".to_owned()),
                    "NAME".to_string() => Value::Text("test".to_owned()),
                    "EXTENSION".to_string() => Value::Text("org".to_owned()),
                ])
            ])),
        );
    }

    #[test]
    fn should_return_error_if_regex_with_named_groups_multi_does_not_match() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.type}".to_string(),
                regex: ExtractorRegex::RegexNamedGroups {
                    regex: r"(?P<PROTOCOL>https?|ftp)://(?P<NAME>[^.\n]+)?".to_string(),
                    all_matches: Some(true),
                },
            },
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("123");

        assert_eq!(extractor.extract(&event, None), None,);
    }

    #[test]
    fn should_return_multi_map_from_tabbed_map() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "${event.payload.table}".to_string(),
                regex: ExtractorRegex::RegexNamedGroups {
                    regex: r#"(?P<PID>[0-9]+)\s+(?P<Time>[0-9:]+)\s+(?P<UserId>[0-9]+)\s+(?P<UserName>\w+)\s+(?P<ServerName>\w+)\s+(?P<Level>[0-9]+)"#
                    .to_string(),
                    all_matches: Some(true),
                }
            },
            &AccessorBuilder::new(),
        )
            .unwrap();

        let mut payload = HashMap::new();
        payload.insert(
            "table".to_owned(),
            Value::Text(
                "483     00:00:00        76      JustynaG        1869AS0071      1
440     00:13:05        629     ArturC  1869AS0031      2
615     00:01:36        240     ArturC  1869AS0071      2
379     00:01:07        30      JoannaS 1869AS0041      3"
                    .to_owned(),
            ),
        );

        let event = InternalEvent::new(Event::new_with_payload("", payload));

        assert_eq!(
            extractor.extract(&event, None).unwrap(),
            Value::Array(vec![
                Value::Map(hashmap![
                    "PID".to_string() => Value::Text("483".to_owned()),
                    "Time".to_string() => Value::Text("00:00:00".to_owned()),
                    "UserId".to_string() => Value::Text("76".to_owned()),
                    "UserName".to_string() => Value::Text("JustynaG".to_owned()),
                    "ServerName".to_string() => Value::Text("1869AS0071".to_owned()),
                    "Level".to_string() => Value::Text("1".to_owned()),
                ]),
                Value::Map(hashmap![
                    "PID".to_string() => Value::Text("440".to_owned()),
                    "Time".to_string() => Value::Text("00:13:05".to_owned()),
                    "UserId".to_string() => Value::Text("629".to_owned()),
                    "UserName".to_string() => Value::Text("ArturC".to_owned()),
                    "ServerName".to_string() => Value::Text("1869AS0031".to_owned()),
                    "Level".to_string() => Value::Text("2".to_owned()),
                ]),
                Value::Map(hashmap![
                    "PID".to_string() => Value::Text("615".to_owned()),
                    "Time".to_string() => Value::Text("00:01:36".to_owned()),
                    "UserId".to_string() => Value::Text("240".to_owned()),
                    "UserName".to_string() => Value::Text("ArturC".to_owned()),
                    "ServerName".to_string() => Value::Text("1869AS0071".to_owned()),
                    "Level".to_string() => Value::Text("2".to_owned()),
                ]),
                Value::Map(hashmap![
                    "PID".to_string() => Value::Text("379".to_owned()),
                    "Time".to_string() => Value::Text("00:01:07".to_owned()),
                    "UserId".to_string() => Value::Text("30".to_owned()),
                    "UserName".to_string() => Value::Text("JoannaS".to_owned()),
                    "ServerName".to_string() => Value::Text("1869AS0041".to_owned()),
                    "Level".to_string() => Value::Text("3".to_owned()),
                ]),
            ]),
        );
    }

    #[test]
    fn should_return_whether_the_regex_has_named_groups() {
        // Arrange
        let no_named_regex = RustRegex::new(r"(https?|ftp)://([^.\n]+).([^.\n]*)?").unwrap();
        let partially_named_regex =
            RustRegex::new(r"(https?|ftp)://([^.\n]+).(?P<PID>[0-9]+)\s+([^.\n]*)?").unwrap();
        let named_regex = RustRegex::new(r#"(?P<PID>[0-9]+)\s+(?P<Time>[0-9:]+)\s+(?P<UserId>[0-9]+)\s+(?P<UserName>\w+)\s+(?P<ServerName>\w+)\s+(?P<Level>[0-9]+)"#).unwrap();

        // Assert
        assert!(!has_named_groups(&no_named_regex));
        assert!(has_named_groups(&partially_named_regex));
        assert!(has_named_groups(&named_regex));
    }

    #[test]
    fn build_should_fail_if_named_match_has_no_named_groups() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor {
                from: "&{event.payload}".to_string(),
                regex: ExtractorRegex::RegexNamedGroups {
                    regex: r"(https?|ftp)://([^.\n]+).([^.\n]*)?".to_string(),
                    all_matches: None,
                },
            },
            &AccessorBuilder::new(),
        );
        assert!(extractor.is_err());
    }

    fn new_event(event_type: &str) -> InternalEvent {
        InternalEvent::new(Event::new(event_type))
    }
}
