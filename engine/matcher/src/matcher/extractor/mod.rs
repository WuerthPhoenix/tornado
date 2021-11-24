//! The extractor module contains the logic to generate variables based on the
//! Rule configuration.
//!
//! An *Extractor* is linked to the "WITH" clause of a Rule and determines the value
//! of dynamically generated variables.

use crate::accessor::{Accessor, AccessorBuilder};
use crate::config::rule::{Extractor, ExtractorRegex, ExtractorRegexType};
use crate::error::MatcherError;
use crate::matcher::modifier::ValueModifier;
use crate::model::{InternalEvent};
use crate::regex::RegexWrapper;
use log::*;
use regex::{Captures, Regex as RustRegex};
use serde_json::{Map, Value};
use tornado_common_api::ValueExt;
use std::collections::BTreeMap;

/// The MatcherExtractor instance builder.
#[derive(Default)]
pub struct MatcherExtractorBuilder {
    accessor_builder: AccessorBuilder,
}

impl MatcherExtractorBuilder {
    /// Returns a new MatcherExtractorBuilder instance.
    pub fn new() -> MatcherExtractorBuilder {
        MatcherExtractorBuilder { accessor_builder: AccessorBuilder::new() }
    }

    /// Returns a specific MatcherExtractor instance based on the matcher.extractor rule configuration.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    ///    use tornado_common_api::{Event, Value, ValueExt, ValueGet};
    ///    use tornado_engine_matcher::matcher::extractor::MatcherExtractorBuilder;
    ///    use tornado_engine_matcher::config::rule::{Extractor, ExtractorRegex, ExtractorRegexType};
    ///    use tornado_engine_matcher::model::InternalEvent;
    ///    use std::collections::BTreeMap;
    ///    use serde_json::{json, Map};
    ///
    ///    let mut extractor_config = BTreeMap::new();
    ///
    ///    extractor_config.insert(
    ///        String::from("extracted_temp"),
    ///        Extractor::Regex(ExtractorRegex {
    ///            from: String::from("${event.type}"),
    ///            regex: ExtractorRegexType::Regex {
    ///                regex: String::from(r"[0-9]+"),
    ///                group_match_idx: Some(0),
    ///                all_matches: None,
    ///            },
    ///            modifiers_post: vec![],
    ///        }),
    ///    );
    ///
    ///    // The matcher_extractor contains the logic to create the "extracted_temp" variable from the ${event.type}.
    ///    // The value of the "extracted_temp" variable is obtained by applying the regular expression "[0-9]+" to
    ///    // the event.type.
    ///    let matcher_extractor = MatcherExtractorBuilder::new().build("rule_name", &extractor_config).unwrap();
    ///
    ///    let event = json!(Event::new("temp=44'C"));
    ///    let mut extracted_vars = Value::Object(Map::new());
    ///    let mut internal_event: InternalEvent = (&event, &mut extracted_vars).into();
    ///    let result = matcher_extractor.process_all(&mut internal_event);
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
        config: &BTreeMap<String, Extractor>,
    ) -> Result<MatcherExtractor, MatcherError> {
        let mut matcher_extractor =
            MatcherExtractor { rule_name: rule_name.to_owned(), extractors: BTreeMap::new() };
        for (key, extractor) in config.iter() {
            matcher_extractor.extractors.insert(
                key.to_owned(),
                ValueExtractor::build(rule_name, key, extractor, &self.accessor_builder)?,
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
    extractors: BTreeMap<String, ValueExtractor>,
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
        event: &mut InternalEvent,
    ) -> Result<(), MatcherError> {
        if !self.extractors.is_empty() {

            
            let mut vars = Map::new();
            for (key, extractor) in &self.extractors {
                let (key, value) = extractor.extract(key, event)?;
                vars.insert(key.to_string(), value);


                if let Some(map) = event.extracted_variables.get_map_mut() {
                    map.insert(self.rule_name.to_string(), Value::Object(vars.clone()));
                } else {
                    return Err(MatcherError::InternalSystemError {
                        message: "MatcherExtractor - process_all - expected a Value::Map".to_owned(),
                    });
                }
            }
            
        }
        Ok(())
    }
}

#[derive(Debug)]
enum ValueExtractor {
    Regex(ValueExtractorRegex),
    Text(ValueExtractorText),
}

#[derive(Debug)]
struct ValueExtractorRegex {
    pub key: String,
    pub regex_extractor: RegexValueExtractor,
    pub modifiers_post: Vec<ValueModifier>,
}

#[derive(Debug)]
struct ValueExtractorText {
    pub key: String,
    pub text: String,
    pub accessor: Accessor,
    pub modifiers_post: Vec<ValueModifier>,
}

impl ValueExtractor {
    pub fn build(
        rule_name: &str,
        key: &str,
        extractor: &Extractor,
        accessor_builder: &AccessorBuilder,
    ) -> Result<ValueExtractor, MatcherError> {
        match extractor {
            Extractor::Regex(extractor) => {
                Ok(Self::Regex(ValueExtractorRegex{
                    key: key.to_owned(),
                    regex_extractor: RegexValueExtractor::build(rule_name, extractor, accessor_builder)?,
                    modifiers_post: ValueModifier::build(rule_name, accessor_builder, &extractor.modifiers_post)?,
                }))
            },
            Extractor::Text(extractor) => {
                Ok(Self::Text(ValueExtractorText{
                    key: key.to_owned(),
                    text: extractor.text.to_owned(),
                    accessor: accessor_builder.build(rule_name, &extractor.text)?,
                    modifiers_post: ValueModifier::build(rule_name, accessor_builder, &extractor.modifiers_post)?,
                }))
            }
        }
    }

    pub fn extract(
        &self,
        variable_name: &str,
        event: &InternalEvent,
    ) -> Result<(&str, Value), MatcherError> {
        match self {
            ValueExtractor::Regex(extractor) => {
                let mut extracted_value = extractor.regex_extractor.extract(variable_name, event)?;
                for modifier in &extractor.modifiers_post {
                    modifier.apply(variable_name, &mut extracted_value, event)?;
                }
                Ok((&extractor.key, extracted_value))
            }
            ValueExtractor::Text(extractor) => {
                let mut extracted_value = extractor.accessor.get(event).ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?.into_owned();                
                for modifier in &extractor.modifiers_post {
                    modifier.apply(variable_name, &mut extracted_value, event)?;
                }
                Ok((&extractor.key, extracted_value))
            }
        }
    }
}

#[derive(Debug)]
enum RegexValueExtractor {
    SingleMatchSingleGroup { regex: RegexWrapper, group_match_idx: usize, target: Accessor },
    AllMatchesSingleGroup { regex: RegexWrapper, group_match_idx: usize, target: Accessor },
    SingleMatchAllGroups { regex: RegexWrapper, target: Accessor },
    AllMatchesAllGroups { regex: RegexWrapper, target: Accessor },
    SingleMatchNamedGroups { regex: RegexWrapper, target: Accessor },
    AllMatchesNamedGroups { regex: RegexWrapper, target: Accessor },
    SingleKeyMatch { regex: RegexWrapper, target: Accessor },
}

impl RegexValueExtractor {
    pub fn build(
        rule_name: &str,
        extractor: &ExtractorRegex,
        accessor: &AccessorBuilder,
    ) -> Result<RegexValueExtractor, MatcherError> {
        let target = accessor.build(rule_name, &extractor.from)?;

        match &extractor.regex {
            ExtractorRegexType::Regex { regex, group_match_idx, all_matches } => {
                let rust_regex = RegexWrapper::new(regex)?;

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
            ExtractorRegexType::RegexNamedGroups { regex, all_matches } => {
                let rust_regex = RegexWrapper::new(regex)?;

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
            ExtractorRegexType::SingleKeyRegex { regex } => {
                let rust_regex = RegexWrapper::new(regex)?;
                Ok(RegexValueExtractor::SingleKeyMatch { regex: rust_regex, target })
            }
        }
    }

    pub fn extract(
        &self,
        variable_name: &str,
        event: &InternalEvent,
    ) -> Result<Value, MatcherError> {
        match self {
            // Note: the non-'multi' implementations could be avoided as they are a particular case of the 'multi' ones;
            // however, we can use an optimized logic if we know beforehand that only the first capture is required
            RegexValueExtractor::SingleMatchSingleGroup { regex, group_match_idx, target } => {
                let cow_value = target.get(event).ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let text = cow_value.get_text().ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let captures = regex.captures(text).ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                captures
                    .get(*group_match_idx)
                    .map(|matched| Value::String(matched.as_str().to_owned()))
                    .ok_or_else(|| MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    })
            }
            RegexValueExtractor::AllMatchesSingleGroup { regex, group_match_idx, target } => {
                let cow_value = target.get(event).ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let text = cow_value.get_text().ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;

                let mut result = vec![];
                for captures in regex.captures_iter(text) {
                    if let Some(value) = captures
                        .get(*group_match_idx)
                        .map(|matched| Value::String(matched.as_str().to_owned()))
                    {
                        result.push(value);
                    } else {
                        return Err(MatcherError::MissingExtractedVariableError {
                            variable_name: variable_name.to_owned(),
                        });
                    }
                }
                if !result.is_empty() {
                    Ok(Value::Array(result))
                } else {
                    Err(MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    })
                }
            }
            RegexValueExtractor::SingleMatchAllGroups { regex, target } => {
                let cow_value = target.get(event).ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let text = cow_value.get_text().ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let captures = regex.captures(text).ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;

                if let Some(groups) = get_indexed_groups(&captures) {
                    Ok(Value::Array(groups))
                } else {
                    Err(MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    })
                }
            }
            RegexValueExtractor::AllMatchesAllGroups { regex, target } => {
                let cow_value = target.get(event).ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let text = cow_value.get_text().ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;

                let mut result = vec![];
                for captures in regex.captures_iter(text) {
                    if let Some(groups) = get_indexed_groups(&captures) {
                        result.push(Value::Array(groups));
                    } else {
                        return Err(MatcherError::MissingExtractedVariableError {
                            variable_name: variable_name.to_owned(),
                        });
                    }
                }
                if !result.is_empty() {
                    Ok(Value::Array(result))
                } else {
                    Err(MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    })
                }
            }
            RegexValueExtractor::SingleMatchNamedGroups { regex, target } => {
                let cow_value = target.get(event).ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let text = cow_value.get_text().ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;

                if let Some(captures) = regex.captures(text) {
                    if let Some(groups) = get_named_groups(&captures, regex) {
                        return Ok(Value::Object(groups));
                    } else {
                        return Err(MatcherError::MissingExtractedVariableError {
                            variable_name: variable_name.to_owned(),
                        });
                    }
                };
                Err(MatcherError::MissingExtractedVariableError {
                    variable_name: variable_name.to_owned(),
                })
            }
            RegexValueExtractor::AllMatchesNamedGroups { regex, target } => {
                let cow_value = target.get(event).ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let text = cow_value.get_text().ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let mut result = vec![];
                for captures in regex.captures_iter(text) {
                    if let Some(groups) = get_named_groups(&captures, regex) {
                        result.push(Value::Object(groups));
                    } else {
                        return Err(MatcherError::MissingExtractedVariableError {
                            variable_name: variable_name.to_owned(),
                        });
                    }
                }
                if !result.is_empty() {
                    Ok(Value::Array(result))
                } else {
                    Err(MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    })
                }
            }
            RegexValueExtractor::SingleKeyMatch { regex, target } => {
                let cow_value = target.get(event).ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let values = cow_value.get_map().ok_or_else(|| {
                    MatcherError::MissingExtractedVariableError {
                        variable_name: variable_name.to_owned(),
                    }
                })?;
                let mut result = None;
                for (key, value) in values {
                    if regex.is_match(key) {
                        if result.is_none() {
                            result = Some(value.clone())
                        } else {
                            return Err(MatcherError::ExtractedVariableError {
                                variable_name: variable_name.to_owned(),
                                message: "Expected exactly one match but found more.".to_owned(),
                            });
                        }
                    }
                }
                result.ok_or_else(|| MatcherError::MissingExtractedVariableError {
                    variable_name: variable_name.to_owned(),
                })
            }
        }
    }
}

fn get_named_groups(captures: &Captures, regex: &RustRegex) -> Option<Map<String, Value>> {
    let mut groups = Map::new();
    for name in regex.capture_names().flatten() {
        if let Some(matched) = captures.name(name) {
            groups.insert(name.to_owned(), Value::String(matched.as_str().to_owned()));
        } else {
            return None;
        }
    }
    Some(groups)
}

fn get_indexed_groups(captures: &Captures) -> Option<Vec<Value>> {
    let mut groups = vec![];
    for capture in captures.iter() {
        if let Some(matched) = capture {
            groups.push(Value::String(matched.as_str().to_owned()))
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
    use crate::config::rule::{ExtractorRegexType, ExtractorText, Modifier};
    use maplit::*;
    use serde_json::json;
    use std::collections::BTreeMap;
    use tornado_common_api::{Event, ValueGet};

    #[test]
    fn should_build_an_extractor() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: "".to_string(),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        );
        assert!(extractor.is_ok());
    }

    #[test]
    fn should_build_an_extractor_with_trim_modifier() {
        // Arrange
        let rule_extractor = ExtractorRegex {
            from: "".to_string(),
            regex: ExtractorRegexType::Regex {
                regex: "".to_string(),
                group_match_idx: Some(0),
                all_matches: None,
            },
            modifiers_post: vec![Modifier::Trim {}],
        };

        // Act
        let extractor =
            ValueExtractor::build("rule_name", "key", &Extractor::Regex(rule_extractor), &AccessorBuilder::new())
                .unwrap();

        // Assert
        match extractor {
            ValueExtractor::Regex(extractor) => {
                assert_eq!(1, extractor.modifiers_post.len());
                match extractor.modifiers_post[0] {
                    ValueModifier::Trim => {},
                    _ => assert!(false)
                }
            },
            _ => assert!(false)
        }
    }

    #[test]
    fn build_should_fail_if_not_valid_regex() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: "[".to_string(),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        );
        assert!(extractor.is_err());
    }

    #[test]
    fn should_match_and_return_group_at_zero() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");


        assert_eq!(
            ("key", Value::String("http://stackoverflow.com/".to_owned())),
            extractor.extract("", &(&event, &mut Value::Null).into()).unwrap()
        );

    }

    #[test]
    fn should_match_and_return_group_at_one() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(1),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!(("key", Value::String("http".to_owned())), extractor.extract("", &(&event, &mut Value::Null).into()).unwrap());
    }

    #[test]
    fn should_match_and_return_group_at_one_multi() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(1),
                    all_matches: Some(true),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/\nftp://test.com");

        assert_eq!(
            ("key", Value::Array(vec![Value::String("http".to_owned()), Value::String("ftp".to_owned()),])),
            extractor.extract("", &(&event, &mut Value::Null).into()).unwrap()
        );
    }

    #[test]
    fn should_match_and_return_group_at_two() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(2),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!(
            ("key", Value::String("stackoverflow.com".to_owned())),
            extractor.extract("", &(&event, &mut Value::Null).into()).unwrap()
        );
    }

    #[test]
    fn should_match_and_return_none_if_not_valid_group() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(10000),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert!(extractor.extract("", &(&event, &mut Value::Null).into()).is_err());
    }

    #[test]
    fn should_match_and_return_none_if_not_valid_group_multi() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(10000),
                    all_matches: Some(true),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert!(extractor.extract("", &(&event, &mut Value::Null).into()).is_err());
    }

    #[test]
    fn should_match_and_return_none_if_not_value_from_event() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.payload.body}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: Some(1),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("");

        assert!(extractor.extract("", &(&event, &mut Value::Null).into()).is_err());
    }

    #[test]
    fn should_extract_all_variables_and_return_true() {
        let mut from_config = BTreeMap::new();

        from_config.insert(
            String::from("extracted_temp"),
            Extractor::Regex(ExtractorRegex {
                from: String::from("${event.type}"),
                regex: ExtractorRegexType::Regex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
        );

        from_config.insert(
            String::from("extracted_text"),
            Extractor::Regex(ExtractorRegex {
                from: String::from("${event.type}"),
                regex: ExtractorRegexType::Regex {
                    regex: String::from(r"[a-z]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
        );

        let extractor = MatcherExtractorBuilder::new().build("rule", &from_config).unwrap();

        let event = new_event("temp=44'C");
        let mut extracted_vars = Value::Object(Map::new());

        extractor.process_all(&mut (&event, &mut extracted_vars).into()).unwrap();

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
        let mut from_config = BTreeMap::new();

        from_config.insert(
            String::from("extracted_temp"),
            Extractor::Regex(ExtractorRegex {
                from: String::from("${event.type}"),
                regex: ExtractorRegexType::Regex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
        );

        from_config.insert(
            String::from("extracted_none"),
            Extractor::Regex(ExtractorRegex {
                from: String::from("${event.payload.nothing}"),
                regex: ExtractorRegexType::Regex {
                    regex: String::from(r"[a-z]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
        );

        let extractor = MatcherExtractorBuilder::new().build("", &from_config).unwrap();

        let event = new_event("temp=44'C");
        let mut extracted_vars = Value::Object(Map::new());

        assert!(extractor.process_all(&mut (&event, &mut extracted_vars).into()).is_err());
    }

    #[test]
    fn should_return_all_matching_groups_if_no_idx() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?".to_string(),
                    group_match_idx: None,
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com/");

        assert_eq!(
            ("key", Value::Array(vec![
                Value::String("http://stackoverflow.com/".to_owned()),
                Value::String("http".to_owned()),
                Value::String("stackoverflow.com".to_owned()),
                Value::String("/".to_owned()),
            ])),
            extractor.extract("", &(&event, &mut Value::Null).into()).unwrap()
        );
    }

    #[test]
    fn should_return_all_matching_groups_multi_if_no_idx() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key1",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^.\n]+).([^.\n]*)?".to_string(),
                    group_match_idx: None,
                    all_matches: Some(true),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com\nftp://test.org");

        assert_eq!(
            extractor.extract("", &(&event, &mut Value::Null).into()).unwrap(),
            ("key1", Value::Array(vec![
                Value::Array(vec![
                    Value::String("http://stackoverflow.com".to_owned()),
                    Value::String("http".to_owned()),
                    Value::String("stackoverflow".to_owned()),
                    Value::String("com".to_owned()),
                ]),
                Value::Array(vec![
                    Value::String("ftp://test.org".to_owned()),
                    Value::String("ftp".to_owned()),
                    Value::String("test".to_owned()),
                    Value::String("org".to_owned()),
                ])
            ])),
        );
    }

    #[test]
    fn should_fail_if_not_all_matching_groups_are_found() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^.\n]+).(/[^.\n]*)?".to_string(),
                    group_match_idx: None,
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow/");

        assert!(extractor.extract("", &(&event, &mut Value::Null).into()).is_err());
    }

    #[test]
    fn should_fail_if_a_matching_group_is_not_found_even_if_regex_matches() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"ab(c)*d".to_string(),
                    group_match_idx: None,
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("abd");

        assert!(extractor.extract("", &(&event, &mut Value::Null).into()).is_err());
    }

    #[test]
    fn should_fail_if_not_all_matching_groups_are_found_with_multi() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(https?|ftp)://([^.\n]+).(/[^.\n]*)?".to_string(),
                    group_match_idx: None,
                    all_matches: Some(true),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow/");

        assert!(extractor.extract("", &(&event, &mut Value::Null).into()).is_err());
    }

    #[test]
    fn should_return_array_of_values_even_with_named_groups() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r"(?P<PROTOCOL>https?|ftp)://(?P<NAME>[^.\n]+).(?P<EXTENSION>[^.\n]*)?"
                        .to_string(),
                    group_match_idx: None,
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com");

        assert_eq!(
            ("key", Value::Array(vec![
                Value::String("http://stackoverflow.com".to_owned()),
                Value::String("http".to_owned()),
                Value::String("stackoverflow".to_owned()),
                Value::String("com".to_owned()),
            ])),
            extractor.extract("", &(&event, &mut Value::Null).into()).unwrap()
        );
    }

    #[test]
    fn should_return_map_with_named_groups() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::RegexNamedGroups {
                    regex: r"(?P<PROTOCOL>https?|ftp)://(?P<NAME>[^.\n]+).(?P<EXTENSION>[^.\n]*)?"
                        .to_string(),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com");

        assert_eq!(
            extractor.extract("", &(&event, &mut Value::Null).into()).unwrap(),
            ("key", json!(btreemap![
                "PROTOCOL".to_string() => Value::String("http".to_owned()),
                "NAME".to_string() => Value::String("stackoverflow".to_owned()),
                "EXTENSION".to_string() => Value::String("com".to_owned()),
            ])),
        );
    }

    #[test]
    fn should_return_map_with_named_groups_ignoring_unnamed_groups() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key3",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::RegexNamedGroups {
                    regex: r"(?P<PROTOCOL>https?|ftp)://([^.\n]+).(?P<EXTENSION>[^.\n]*)?"
                        .to_string(),
                    all_matches: Some(false),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com");

        assert_eq!(
            extractor.extract("", &(&event, &mut Value::Null).into()).unwrap(),
            ("key3", json!(btreemap![
                "PROTOCOL".to_string() => Value::String("http".to_owned()),
                "EXTENSION".to_string() => Value::String("com".to_owned()),
            ])),
        );
    }

    #[test]
    fn should_return_error_if_regex_with_named_groups_does_not_match() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::RegexNamedGroups {
                    regex: r"(?P<PROTOCOL>https?|ftp)://(?P<NAME>[^.\n]+)?".to_string(),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("123");

        assert!(extractor.extract("", &(&event, &mut Value::Null).into()).is_err());
    }

    #[test]
    fn should_return_multi_map_with_named_groups() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::RegexNamedGroups {
                    regex: r"(?P<PROTOCOL>https?|ftp)://(?P<NAME>[^.\n]+).(?P<EXTENSION>[^.\n]*)?"
                        .to_string(),
                    all_matches: Some(true),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("http://stackoverflow.com\nftp://test.org");

        assert_eq!(
            extractor.extract("", &(&event, &mut Value::Null).into()).unwrap(),
            ("key", Value::Array(vec![
                json!(btreemap![
                    "PROTOCOL".to_string() => Value::String("http".to_owned()),
                    "NAME".to_string() => Value::String("stackoverflow".to_owned()),
                    "EXTENSION".to_string() => Value::String("com".to_owned()),
                ]),
                json!(btreemap![
                    "PROTOCOL".to_string() => Value::String("ftp".to_owned()),
                    "NAME".to_string() => Value::String("test".to_owned()),
                    "EXTENSION".to_string() => Value::String("org".to_owned()),
                ])
            ])),
        );
    }

    #[test]
    fn should_return_error_if_regex_with_named_groups_multi_does_not_match() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::RegexNamedGroups {
                    regex: r"(?P<PROTOCOL>https?|ftp)://(?P<NAME>[^.\n]+)?".to_string(),
                    all_matches: Some(true),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("123");

        assert!(extractor.extract("", &(&event, &mut Value::Null).into()).is_err());
    }

    #[test]
    fn should_return_multi_map_from_tabbed_map() {
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.payload.table}".to_string(),
                regex: ExtractorRegexType::RegexNamedGroups {
                    regex: r#"(?P<PID>[0-9]+)\s+(?P<Time>[0-9:]+)\s+(?P<UserId>[0-9]+)\s+(?P<UserName>\w+)\s+(?P<ServerName>\w+)\s+(?P<Level>[0-9]+)"#
                    .to_string(),
                    all_matches: Some(true),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
            .unwrap();

        let mut payload = Map::new();
        payload.insert(
            "table".to_owned(),
            Value::String(
                "483     00:00:00        76      JustynaG        1869AS0071      1
440     00:13:05        629     ArturC  1869AS0031      2
615     00:01:36        240     ArturC  1869AS0071      2
379     00:01:07        30      JoannaS 1869AS0041      3"
                    .to_owned(),
            ),
        );

        let event = json!(Event::new_with_payload("", payload));

        assert_eq!(
            extractor.extract("", &(&event, &mut Value::Null).into()).unwrap(),
            ("key", Value::Array(vec![
                json!(btreemap![
                    "PID".to_string() => Value::String("483".to_owned()),
                    "Time".to_string() => Value::String("00:00:00".to_owned()),
                    "UserId".to_string() => Value::String("76".to_owned()),
                    "UserName".to_string() => Value::String("JustynaG".to_owned()),
                    "ServerName".to_string() => Value::String("1869AS0071".to_owned()),
                    "Level".to_string() => Value::String("1".to_owned()),
                ]),
                json!(btreemap![
                    "PID".to_string() => Value::String("440".to_owned()),
                    "Time".to_string() => Value::String("00:13:05".to_owned()),
                    "UserId".to_string() => Value::String("629".to_owned()),
                    "UserName".to_string() => Value::String("ArturC".to_owned()),
                    "ServerName".to_string() => Value::String("1869AS0031".to_owned()),
                    "Level".to_string() => Value::String("2".to_owned()),
                ]),
                json!(btreemap![
                    "PID".to_string() => Value::String("615".to_owned()),
                    "Time".to_string() => Value::String("00:01:36".to_owned()),
                    "UserId".to_string() => Value::String("240".to_owned()),
                    "UserName".to_string() => Value::String("ArturC".to_owned()),
                    "ServerName".to_string() => Value::String("1869AS0071".to_owned()),
                    "Level".to_string() => Value::String("2".to_owned()),
                ]),
                json!(btreemap![
                    "PID".to_string() => Value::String("379".to_owned()),
                    "Time".to_string() => Value::String("00:01:07".to_owned()),
                    "UserId".to_string() => Value::String("30".to_owned()),
                    "UserName".to_string() => Value::String("JoannaS".to_owned()),
                    "ServerName".to_string() => Value::String("1869AS0041".to_owned()),
                    "Level".to_string() => Value::String("3".to_owned()),
                ]),
            ])),
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
            &Extractor::Regex(ExtractorRegex {
                from: "&{event.payload}".to_string(),
                regex: ExtractorRegexType::RegexNamedGroups {
                    regex: r"(https?|ftp)://([^.\n]+).([^.\n]*)?".to_string(),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        );
        assert!(extractor.is_err());
    }

    #[test]
    fn build_should_fail_if_single_key_match_has_not_valid_regex() {
        // Arrange & Act
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::SingleKeyRegex { regex: "[".to_string() },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        );

        // Assert
        assert!(extractor.is_err());
    }

    #[test]
    fn build_should_succeed_if_single_key_match_has_valid_regex() {
        // Arrange & Act
        let regex = r"(https?|ftp)://([^/\r\n]+)(/[^\r\n]*)?";
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.type}".to_string(),
                regex: ExtractorRegexType::SingleKeyRegex { regex: regex.to_string() },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        // Assert
        match extractor {
            ValueExtractor::Regex(extractor) => {
                assert_eq!("key", &extractor.key);
                match extractor.regex_extractor {
                    RegexValueExtractor::SingleKeyMatch { .. } => {}
                    _ => assert!(false),
                }
            },
            _ => assert!(false)
        }
    }

    #[test]
    fn single_key_match_should_match_single_entry() {
        // Arrange
        let mut oids = Map::new();
        oids.insert(
            "MWRM2-NMS-MIB::netmasterAlarmNeIpv4Address.201476692".to_owned(),
            Value::String("0".to_owned()),
        );
        oids.insert(
            "MWRM2-NMS-MIB::netmasterAlarmNeIpv6Address.201476692".to_owned(),
            Value::String("1".to_owned()),
        );
        oids.insert(
            "MWRM2-NMS-MIB::netmasterAlarm.201476692".to_owned(),
            Value::String("2".to_owned()),
        );
        oids.insert(
            "MWRM2-NMS-MIB::netmasterAlarmStatus.201476692".to_owned(),
            Value::String("3".to_owned()),
        );
        let mut payload = Map::new();
        payload.insert("oids".to_owned(), Value::Object(oids));

        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.payload.oids}".to_string(),
                regex: ExtractorRegexType::SingleKeyRegex {
                    regex: r#"MWRM2-NMS-MIB::netmasterAlarmNeIpv6Address\."#.to_string(),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = json!(Event::new_with_payload("event", payload));

        // Act
        let result = extractor.extract("var", &(&event, &mut Value::Null).into());

        // Assert
        assert!(result.is_ok());
        assert_eq!(("key", Value::String("1".to_owned())), result.unwrap());
    }

    #[test]
    fn single_key_match_should_fail_if_no_match() {
        // Arrange
        let mut oids = Map::new();
        oids.insert(
            "MWRM2-NMS-MIB::netmasterAlarm.201476692".to_owned(),
            Value::String("2".to_owned()),
        );
        oids.insert(
            "MWRM2-NMS-MIB::netmasterAlarmStatus.201476692".to_owned(),
            Value::String("3".to_owned()),
        );
        let mut payload = Map::new();
        payload.insert("oids".to_owned(), Value::Object(oids));

        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.payload.oids}".to_string(),
                regex: ExtractorRegexType::SingleKeyRegex {
                    regex: r#"MWRM2-NMS-MIB::netmasterAlarmNeIpv6Address\."#.to_string(),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = json!(Event::new_with_payload("event", payload));

        // Act
        let result = extractor.extract("var", &(&event, &mut Value::Null).into());

        // Assert
        assert!(result.is_err());
        assert_eq!(
            Err(MatcherError::MissingExtractedVariableError { variable_name: "var".to_owned() }),
            result
        )
    }

    #[test]
    fn single_key_match_should_fail_if_more_than_one_match() {
        // Arrange
        let mut oids = Map::new();
        oids.insert(
            "MWRM2-NMS-MIB::netmasterAlarmNeIpv4Address.201476692".to_owned(),
            Value::String("0".to_owned()),
        );
        oids.insert(
            "MWRM2-NMS-MIB::netmasterAlarmNeIpv6Address.201476692".to_owned(),
            Value::String("1".to_owned()),
        );
        oids.insert(
            "MWRM2-NMS-MIB::netmasterAlarm.201476692".to_owned(),
            Value::String("2".to_owned()),
        );
        oids.insert(
            "MWRM2-NMS-MIB::netmasterAlarmStatus.201476692".to_owned(),
            Value::String("3".to_owned()),
        );
        let mut payload = Map::new();
        payload.insert("oids".to_owned(), Value::Object(oids));

        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.payload.oids}".to_string(),
                regex: ExtractorRegexType::SingleKeyRegex {
                    regex: r#"MWRM2-NMS-MIB::netmasterAlarmNe[a-z.A-Z0.9]*"#.to_string(),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = json!(Event::new_with_payload("event", payload));

        // Act
        let result = extractor.extract("var", &(&event, &mut Value::Null).into());

        // Assert
        assert!(result.is_err());
        assert_eq!(
            Err(MatcherError::ExtractedVariableError {
                variable_name: "var".to_owned(),
                message: "Expected exactly one match but found more.".to_owned(),
            }),
            result
        )
    }

    #[test]
    fn single_key_match_should_fail_if_value_is_not_a_map() {
        // Arrange
        let mut payload = Map::new();
        payload.insert(
            "oids".to_owned(),
            Value::Array(vec![
                Value::String("MWRM2-NMS-MIB::netmasterAlarm.201476692".to_owned()),
                Value::String("MWRM2-NMS-MIB::netmasterAlarmStatus.201476692".to_owned()),
            ]),
        );

        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.payload.oids}".to_string(),
                regex: ExtractorRegexType::SingleKeyRegex {
                    regex: r#"MWRM2-NMS-MIB::netmasterAlarmNeIpv6Address\."#.to_string(),
                },
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = json!(Event::new_with_payload("event", payload));

        // Act
        let result = extractor.extract("var", &(&event, &mut Value::Null).into());

        // Assert
        assert!(result.is_err());
        assert_eq!(
            Err(MatcherError::MissingExtractedVariableError { variable_name: "var".to_owned() }),
            result
        )
    }

    #[test]
    fn should_apply_the_trim_post_modifier() {
        // Arrange
        let mut oids = Map::new();
        oids.insert("1".to_owned(), Value::String("Hello not to be trimmed".to_owned()));
        oids.insert("2".to_owned(), Value::String("Hello to be trimmed  ".to_owned()));

        let mut payload = Map::new();
        payload.insert("oids".to_owned(), Value::Object(oids));

        let extractor_1 = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.payload.oids}".to_string(),
                regex: ExtractorRegexType::SingleKeyRegex { regex: r#"1"#.to_string() },
                modifiers_post: vec![Modifier::Trim {}, Modifier::Trim {}],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let extractor_2 = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.payload.oids.2}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r#".*"#.to_string(),
                    all_matches: Some(false),
                    group_match_idx: Some(0),
                },
                modifiers_post: vec![Modifier::Trim {}],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = json!(Event::new_with_payload("event", payload));

        // Act
        let result_1 = extractor_1.extract("var", &(&event, &mut Value::Null).into()).unwrap();
        let result_2 = extractor_2.extract("var", &(&event, &mut Value::Null).into()).unwrap();

        // Assert
        assert_eq!(("key", Value::String("Hello not to be trimmed".to_owned())), result_1);
        assert_eq!(("key", Value::String("Hello to be trimmed".to_owned())), result_2);
    }

    #[test]
    fn extractor_should_fail_if_trim_post_modifier_is_not_applied_to_string() {
        // Arrange
        let mut oids = Map::new();
        oids.insert("2".to_owned(), Value::String("Hello to be trimmed  ".to_owned()));

        let mut payload = Map::new();
        payload.insert("oids".to_owned(), Value::Object(oids));

        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.payload.oids.2}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r#".*"#.to_string(),
                    all_matches: Some(true),
                    group_match_idx: None,
                },
                modifiers_post: vec![Modifier::Trim {}],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = json!(Event::new_with_payload("event", payload));

        // Act
        let result = extractor.extract("var", &(&event, &mut Value::Null).into());

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_apply_chained_modifiers() {
        // Arrange
        let mut oids = Map::new();
        oids.insert(
            "1".to_owned(),
            Value::String("    Hello to be trimmed AND LOWERCASED    ".to_owned()),
        );

        let mut payload = Map::new();
        payload.insert("oids".to_owned(), Value::Object(oids));

        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Regex(ExtractorRegex {
                from: "${event.payload.oids.1}".to_string(),
                regex: ExtractorRegexType::Regex {
                    regex: r#".*"#.to_string(),
                    all_matches: Some(false),
                    group_match_idx: Some(0),
                },
                modifiers_post: vec![
                    Modifier::Trim {},
                    Modifier::Lowercase {},
                    Modifier::ReplaceAll {
                        find: "and".to_owned(),
                        replace: "replaced_and".to_owned(),
                        is_regex: false,
                    },
                ],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = json!(Event::new_with_payload("event", payload));

        // Act
        let result = extractor.extract("var", &(&event, &mut Value::Null).into()).unwrap();

        // Assert
        assert_eq!(("key", Value::String("hello to be trimmed replaced_and lowercased".to_owned())), result);
    }

    #[test]
    fn text_extractor_should_return_event_type() {
        // Arrange
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Text(ExtractorText {
                text: "${event.type}".to_owned(),
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("EVENT__TYPE");

        // Act
        let (var, value) = extractor.extract("", &(&event, &mut Value::Null).into()).unwrap();

        // Assert
        assert_eq!("key", var);
        assert_eq!("EVENT__TYPE", &value);
    }

    #[test]
    fn text_extractor_should_allow_string_interpolation() {
        // Arrange
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Text(ExtractorText {
                text: "The type is: ${event.type} and a value is ${event.payload.some}".to_owned(),
                modifiers_post: vec![],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let mut payload = Map::new();
        payload.insert("some".to_owned(), Value::String("A VALUE".to_owned()));
        let event = Event::new_with_payload("SOME_TYPE".to_owned(), payload);
        
        // Act
        let (var, value) = extractor.extract("", &(&json!(event), &mut Value::Null).into()).unwrap();

        // Assert
        assert_eq!("key", var);
        assert_eq!("The type is: SOME_TYPE and a value is A VALUE", &value);
    }


    #[test]
    fn text_extractor_should_apply_post_modifiers() {
        // Arrange
        let extractor = ValueExtractor::build(
            "rule_name",
            "key",
            &Extractor::Text(ExtractorText {
                text: "${event.type}".to_owned(),
                modifiers_post: vec![Modifier::Lowercase {}],
            }),
            &AccessorBuilder::new(),
        )
        .unwrap();

        let event = new_event("EVENT__TYPE");

        // Act
        let (var, value) = extractor.extract("", &(&event, &mut Value::Null).into()).unwrap();

        // Assert
        assert_eq!("key", var);
        assert_eq!("event__type", &value);
    }

    #[test]
    fn text_extractor_should_use_previously_extracted_variables() {
        // Arrange
        let mut from_config = BTreeMap::new();

        from_config.insert(
            String::from("a_temperature"),
            Extractor::Text(ExtractorText {
                text: "${event.payload.temperature}".to_owned(),
                modifiers_post: vec![],
            }),
        );

        from_config.insert(
            String::from("decorated"),
            Extractor::Text(ExtractorText {
                text: "The temperature is: ${_variables.rule.a_temperature}".to_owned(),
                modifiers_post: vec![],
            }),
        );

        let rule_name = "rule";
        let extractor = MatcherExtractorBuilder::new().build(rule_name, &from_config).unwrap();

        let mut payload = Map::new();
        payload.insert("temperature".to_owned(), Value::String("41".to_owned()));
        let event = json!(Event::new_with_payload("SOME_TYPE".to_owned(), payload));

        let mut extracted_vars = Value::Object(Map::new());

        // Act
        extractor.process_all(&mut (&event, &mut extracted_vars).into()).unwrap();

        // Assert
        assert_eq!(
            "41",
            extracted_vars.get_from_map(rule_name).unwrap().get_from_map("a_temperature").unwrap()
        );
        assert_eq!(
            "The temperature is: 41",
            extracted_vars.get_from_map(rule_name).unwrap().get_from_map("decorated").unwrap()
        );
    }

    fn new_event(event_type: &str) -> Value {
        json!(Event::new(event_type))
    }
}
