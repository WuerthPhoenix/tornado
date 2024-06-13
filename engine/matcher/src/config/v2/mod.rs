mod error;

use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::config::v2::error::DeserializationError;
pub use crate::config::v2::error::MatcherConfigError;
use crate::config::{Defaultable, MatcherConfig, MatcherConfigReader};
use crate::error::MatcherError;
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use log::{info, warn};
use monostate::MustBe;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub struct FsMatcherConfigManagerV2<'config> {
    root_path: &'config Path,
    drafts_path: &'config Path,
}

impl FsMatcherConfigManagerV2<'_> {
    pub fn new<'config, P: Into<&'config Path>>(
        root_path: P,
        drafts_path: P,
    ) -> FsMatcherConfigManagerV2<'config> {
        FsMatcherConfigManagerV2 { root_path: root_path.into(), drafts_path: drafts_path.into() }
    }
}

#[derive(Copy, Clone)]
pub enum ConfigType {
    Root,
    Filter,
    Ruleset,
}

impl ConfigType {
    pub fn filename(&self) -> &'static str {
        match self {
            ConfigType::Root => "version.json",
            ConfigType::Filter => "filter.json",
            ConfigType::Ruleset => "ruleset.json",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatcherConfigFilter {
    #[serde(rename = "type")]
    node_type: MustBe!("filter"),
    name: String,
    #[serde(flatten)]
    filter: Filter,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatcherConfigRuleset {
    #[serde(rename = "type")]
    node_type: MustBe!("ruleset"),
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    version: MustBe!("2.0"),
}

// ToDo: Improve the error handling in NEPROD-1658
#[async_trait::async_trait(?Send)]
impl MatcherConfigReader for FsMatcherConfigManagerV2<'_> {
    async fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
        Ok(read_config_from_root_dir(self.root_path).await?)
    }
}

async fn read_config_from_root_dir(root_dir: &Path) -> Result<MatcherConfig, MatcherConfigError> {
    let version_file = {
        let mut path = root_dir.to_path_buf();
        path.push(ConfigType::Root.filename());
        path
    };
    let _: Version = parse_config_from_file(&version_file).await?;
    let nodes = read_child_nodes_from_dir(root_dir, ConfigType::Root).await?;
    Ok(MatcherConfig::Filter {
        name: "root".to_string(),
        filter: Filter {
            active: true,
            description: "An implicit filter that allows all events".to_owned(),
            filter: Defaultable::Default {},
        },
        nodes,
    })
}

#[async_recursion::async_recursion]
async fn read_child_nodes_from_dir(
    dir: &Path,
    config_type: ConfigType,
) -> Result<Vec<MatcherConfig>, MatcherConfigError> {
    let mut root_dir_iter = match tokio::fs::read_dir(dir).await {
        Ok(root_dir_iter) => root_dir_iter,
        Err(error) => {
            return Err(MatcherConfigError::DirReadError { path: PathBuf::from(dir), error })
        }
    };

    let mut processing_nodes_futures = FuturesOrdered::new();
    loop {
        let dir_entry = match root_dir_iter.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(error) => {
                return Err(MatcherConfigError::DirReadError { path: PathBuf::from(dir), error })
            }
        };

        let file_type = match dir_entry.file_type().await {
            Ok(file_type) => file_type,
            Err(error) => {
                return Err(MatcherConfigError::DirReadError { path: PathBuf::from(dir), error })
            }
        };

        if file_type.is_file() {
            if dir_entry.file_name() == config_type.filename() {
                continue;
            }

            if dir_entry.path().extension() != Some(OsStr::new("json")) {
                info!("Ignoring file [{}] as it is not a config file.", dir_entry.path().display());
                continue;
            }

            return Err(MatcherConfigError::UnexpectedFile { path: dir_entry.path(), config_type });
        }

        if !file_type.is_dir() {
            warn!("Ignoring file [{}] because of unknown type.", dir_entry.path().display());
            continue;
        }

        // Push future to be processed concurrently.
        let child_node = read_node_from_dir(dir_entry.path());
        processing_nodes_futures.push_back(child_node)
    }

    let mut processing_nodes = vec![];
    while let Some(result) = processing_nodes_futures.next().await {
        processing_nodes.push(result?);
    }

    Ok(processing_nodes)
}

async fn read_node_from_dir<T: AsRef<Path>>(dir: T) -> Result<MatcherConfig, MatcherConfigError> {
    let dir = dir.as_ref();
    match read_filter_from_dir(dir).await {
        Ok(config) => return Ok(config),
        Err(MatcherConfigError::FileNotFound { .. }) => {}
        Err(error) => return Err(error),
    }

    match read_ruleset_from_dir(dir).await {
        Ok(config) => Ok(config),
        Err(MatcherConfigError::FileNotFound { .. }) => {
            Err(MatcherConfigError::UnknownNodeDir { path: dir.to_path_buf() })
        }
        Err(error) => Err(error),
    }
}

async fn read_filter_from_dir(dir: &Path) -> Result<MatcherConfig, MatcherConfigError> {
    let dir_type = ConfigType::Filter;
    let config_file_path = {
        let mut path = PathBuf::from(dir);
        path.push(dir_type.filename());
        path
    };

    let node: MatcherConfigFilter = parse_config_from_file(&config_file_path).await?;
    let child_nodes = read_child_nodes_from_dir(dir, dir_type).await?;

    Ok(MatcherConfig::Filter { name: node.name, filter: node.filter, nodes: child_nodes })
}

async fn read_ruleset_from_dir(dir: &Path) -> Result<MatcherConfig, MatcherConfigError> {
    let dir_type = ConfigType::Ruleset;
    let config_file_path = {
        let mut path = PathBuf::from(dir);
        path.push(dir_type.filename());
        path
    };

    let rules_dir_path = {
        let mut path = PathBuf::from(dir);
        path.push("rules");
        path
    };

    let ruleset: MatcherConfigRuleset = parse_config_from_file(&config_file_path).await?;
    let rules = read_rules_from_dir(&rules_dir_path).await?;

    Ok(MatcherConfig::Ruleset { name: ruleset.name, rules })
}

async fn read_rules_from_dir(dir: &Path) -> Result<Vec<Rule>, MatcherConfigError> {
    let mut root_dir_iter = match tokio::fs::read_dir(dir).await {
        Ok(root_dir_iter) => root_dir_iter,
        Err(error) => {
            return Err(MatcherConfigError::DirReadError { path: PathBuf::from(dir), error })
        }
    };

    let mut rules = vec![];
    loop {
        let dir_entry = match root_dir_iter.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(error) => {
                return Err(MatcherConfigError::DirReadError { path: PathBuf::from(dir), error })
            }
        };

        let file_type = match dir_entry.file_type().await {
            Ok(file_type) => file_type,
            Err(error) => {
                return Err(MatcherConfigError::DirReadError { path: PathBuf::from(dir), error })
            }
        };

        if !file_type.is_file() {
            warn!("Ignoring directory entry [{}] as it is not a file.", dir_entry.path().display());
            continue;
        }

        let rule: Rule = parse_config_from_file(&dir_entry.path()).await?;
        rules.push(rule)
    }

    Ok(rules)
}

async fn parse_config_from_file<Data: DeserializeOwned>(
    path: &Path,
) -> Result<Data, MatcherConfigError> {
    let content = match tokio::fs::read(&path).await {
        Ok(content) => content,
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                return Err(MatcherConfigError::FileNotFound { path: path.to_path_buf() });
            }

            return Err(MatcherConfigError::FileIoError { path: path.to_path_buf(), error });
        }
    };

    let jd = &mut serde_json::Deserializer::from_slice(&content);
    match serde_path_to_error::deserialize(jd) {
        Ok(result) => Ok(result),
        Err(error) => Err(MatcherConfigError::DeserializationError {
            file: path.to_path_buf(),
            error: parse_serde_errors(error),
        }),
    }
}

// I know this is not pretty, but serde does not model the custom errors to give good user feedback.
// Since we know we are using serde_json and the errors are generated from serde_derive, which is
// on a stable 1.0 release, we can use regex to parse the data from the error message.
fn parse_serde_errors(
    error: serde_path_to_error::Error<serde_json::Error>,
) -> DeserializationError {
    let json_error = error.inner();
    if json_error.is_syntax() {
        return DeserializationError::FormatError {
            line: json_error.line(),
            column: json_error.column(),
        };
    }

    let path = format!("{}", error.path());
    let error = format!("{}", json_error);

    // Determine whether the error is due to a type missmatch.
    // Example error:
    //      filter: invalid type: integer `2`, expected struct Filter at line 3 column 19
    let type_error_regex = Regex::new(
        "^(?<FIELD_NAME>.+?): invalid type: (?<ACTUAL_TYPE>.+?) .+, expected (?<EXPECTED_TYPE>.+?) at line [0-9]+ column [0-9]+$",
    )
        .expect("Static regex should be valid");
    if let Some(result) = type_error_regex.captures(&error) {
        let actual_type = result.name("ACTUAL_TYPE");
        let expected_type = result.name("EXPECTED_TYPE");

        if let (Some(actual_type), Some(expected_type)) = (actual_type, expected_type) {
            return DeserializationError::TypeError {
                path,
                actual_type: actual_type.as_str().to_string(),
                expected_type: expected_type.as_str().to_string(),
            };
        }
    }

    // We disallow unknown fields during parsing, to avoid common human error. Determine if
    // the error occurred due to a not valid field. Example error:
    //      pippo: unknown field `pippo`, expected one of `type`, `name`, `filter` at line 3 column 15
    let unknown_field_error_regex = Regex::new(
        "^(?<FIELD_NAME>.+?): unknown field `(?<FIELD_NAME_2>.+?)`, expected one of (?:`.+?`, )+`.+?` at line [0-9]+ column [0-9]+$",
    )
        .expect("Static regex should be valid");
    if unknown_field_error_regex.is_match(&error) {
        return DeserializationError::UnknownField { path };
    }

    // Determine whether an error occurred, due to a missing field.
    // Example Error:
    //      missing field `name` at line 3 column 5
    let missing_field_error =
        Regex::new("^missing field `(?<NAME>.+?)` at line [0-9]+ column [0-9]+$")
            .expect("Static regex should be valid");
    if let Some(captures) = missing_field_error.captures(&error) {
        if let Some(field) = captures.name("NAME") {
            return DeserializationError::MissingField { path, field: field.as_str().to_string() };
        }
    }

    // Determine whether an error occurred due to a missmatch in expected values.
    // Example error:
    //      type: invalid value: string "ruleset", expected string "filter" at line 2 column 25
    let expected = Regex::new(
        "^(?<FIELD_NAME>.+?): invalid value: (?<ACTUAL_TYPE>.+?) (?<ACTUAL_CONTENT>.+?), expected (?<EXPECTED_TYPE>.+?) (?<EXPECTED_CONTENT>.+?) at line [0-9]+ column [0-9]+$",
    )
        .expect("Static regex should be valid");
    if let Some(captures) = missing_field_error.captures(&error) {
        let actual_type = captures.name("ACTUAL_TYPE");
        let actual_content = captures.name("ACTUAL_CONTENT");
        let expected_type = captures.name("EXPECTED_TYPE");
        let expected_content = captures.name("EXPECTED_CONTENT");

        if let (Some(found_type), Some(found), Some(expected_type), Some(expected)) =
            (actual_type, actual_content, expected_type, expected_content)
        {
            return DeserializationError::InvalidField {
                path,
                found: found.as_str().to_string(),
                found_type: found_type.as_str().to_string(),
                expected: expected.as_str().to_string(),
                expected_type: expected_type.as_str().to_string(),
            };
        }
    }

    // If none of the above match, return a generic error.
    return DeserializationError::GenericError { error };
}

#[test]
fn deserialize_test() {
    let json = r#"{
        "type": "ruleset"
    }"#;

    let jd = &mut serde_json::Deserializer::from_slice(json.as_bytes());
    let error: Result<MatcherConfigFilter, _> = serde_path_to_error::deserialize(jd);

    println!("{}", error.unwrap_err())
}
