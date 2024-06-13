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
use tokio::fs::DirEntry;

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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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
    #[allow(dead_code)]
    node_type: MustBe!("filter"),
    name: String,
    #[serde(flatten)]
    filter: Filter,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatcherConfigRuleset {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    node_type: MustBe!("ruleset"),
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    #[allow(dead_code)]
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
    let dir_entries = gather_dir_entries(dir).await?;

    let mut processing_nodes_futures = FuturesOrdered::new();
    for dir_entry in dir_entries {
        let file_type = match dir_entry.file_type().await {
            Ok(file_type) => file_type,
            Err(error) => {
                return Err(MatcherConfigError::DirIoError { path: PathBuf::from(dir), error })
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
    let dir_entries = gather_dir_entries(dir).await?;

    let mut rules = vec![];
    for dir_entry in dir_entries {
        let file_type = match dir_entry.file_type().await {
            Ok(file_type) => file_type,
            Err(error) => {
                return Err(MatcherConfigError::DirIoError { path: PathBuf::from(dir), error })
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
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(content) => content,
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                return Err(MatcherConfigError::FileNotFound { path: path.to_path_buf() });
            }

            return Err(MatcherConfigError::FileIoError { path: path.to_path_buf(), error });
        }
    };

    let json = content.trim();
    let jd = &mut serde_json::Deserializer::from_str(json);
    match serde_path_to_error::deserialize(jd) {
        Ok(result) => Ok(result),
        Err(error) => Err(MatcherConfigError::DeserializationError {
            file: path.to_path_buf(),
            error: parse_serde_errors(error),
        }),
    }
}

async fn gather_dir_entries(dir: &Path) -> Result<Vec<DirEntry>, MatcherConfigError> {
    let mut root_dir_iter = match tokio::fs::read_dir(dir).await {
        Ok(root_dir_iter) => root_dir_iter,
        Err(error) => {
            return Err(MatcherConfigError::DirIoError { path: PathBuf::from(dir), error })
        }
    };

    let mut dir_entries = vec![];
    loop {
        match root_dir_iter.next_entry().await {
            Ok(Some(entry)) => dir_entries.push(entry),
            Ok(None) => break,
            Err(error) => {
                return Err(MatcherConfigError::DirIoError { path: PathBuf::from(dir), error })
            }
        };
    }

    dir_entries.sort_by_key(DirEntry::path);
    Ok(dir_entries)
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
    //      invalid type: integer `2`, expected a string at line 3 column 11
    let type_error_regex = Regex::new(
        "^invalid type: (?<ACTUAL_TYPE>.+?) .+, expected (?:a )?(?<EXPECTED_TYPE>.+?) at line [0-9]+ column [0-9]+$",
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
    //      unknown field `pippo` at line 3 column 15
    let unknown_field_error_regex = Regex::new(
        "^unknown field `(?<FIELD_NAME>.+?)`(?:, expected one of (?:`.+?`, )+`.+?`)? at line [0-9]+ column [0-9]+$",
    )
        .expect("Static regex should be valid");
    if let Some(captures) = unknown_field_error_regex.captures(&error) {
        if let Some(field) = captures.name("FIELD_NAME") {
            return DeserializationError::UnknownField { path, field: field.as_str().to_string() };
        }
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
    //      invalid value: string "ruleset", expected string "filter" at line 2 column 19
    let expected_value_regex = Regex::new(
        "^invalid value: (?<ACTUAL_TYPE>.+?) (?<ACTUAL_CONTENT>.+?), expected (?<EXPECTED_TYPE>.+?) (?<EXPECTED_CONTENT>.+?) at line [0-9]+ column [0-9]+$",
    )
        .expect("Static regex should be valid");
    if let Some(captures) = expected_value_regex.captures(&error) {
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

#[cfg(test)]
mod tests {
    use crate::config::filter::Filter;
    use crate::config::rule::{ConfigAction, Constraint, Operator, Rule};
    use crate::config::v2::error::DeserializationError;
    use crate::config::v2::{
        parse_config_from_file, read_config_from_root_dir, read_filter_from_dir,
        read_node_from_dir, read_rules_from_dir, read_ruleset_from_dir, ConfigType,
        MatcherConfigError, MatcherConfigFilter, MatcherConfigRuleset,
    };
    use crate::config::{Defaultable, MatcherConfig};
    use monostate::MustBe;
    use std::path::Path;
    use tornado_common_api::Value;

    const TEST_CONFIG_DIR: &str = "./test_resources/v2/test_config/";
    const TEST_BROKEN_CONFIG_DIR: &str = "./test_resources/v2/erroneous_configs/";

    #[tokio::test]
    async fn should_parse_filter_from_file() {
        let path = String::from(TEST_CONFIG_DIR) + "master/filter.json";
        let config: MatcherConfigFilter = parse_config_from_file(Path::new(&path)).await.unwrap();

        match config {
            MatcherConfigFilter {
                node_type: MustBe!("filter"),
                name,
                filter:
                    Filter { active: true, filter: Defaultable::Value(Operator::And { .. }), .. },
            } => {
                assert_eq!("master", name);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_parse_ruleset_from_file() {
        let path = String::from(TEST_CONFIG_DIR) + "tenant_a/snmp_logger/ruleset.json";
        let config: MatcherConfigRuleset = parse_config_from_file(Path::new(&path)).await.unwrap();

        match config {
            MatcherConfigRuleset { node_type: MustBe!("ruleset"), name } => {
                assert_eq!("snmp_logger", name);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_parse_rule_from_file() {
        let path = String::from(TEST_CONFIG_DIR)
            + "tenant_a/snmp_logger/rules/000000010_log_internal_snmp_traps.json";
        let config: Rule = parse_config_from_file(Path::new(&path)).await.unwrap();

        let actions = match config {
            Rule {
                name,
                do_continue: true,
                active: true,
                constraint: Constraint { where_operator: Some(Operator::And { .. }), with },
                actions,
                ..
            } => {
                assert_eq!("log_internal_snmp_traps", name);
                assert!(with.is_empty());
                actions
            }
            result => panic!("{:#?}", result),
        };

        match actions.as_slice() {
            [ConfigAction { id, payload }] => {
                assert_eq!("logger", id);
                assert!(payload.contains_key("event"))
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_read_all_rules_from_directory() {
        let path = String::from(TEST_CONFIG_DIR) + "tenant_a/snmp_logger/rules";
        let config = read_rules_from_dir(Path::new(&path)).await.unwrap();

        let (rule1, rule2) = match config.as_slice() {
            [rule1 @ Rule { name: name_rule_1, do_continue: true, active: true, .. }, rule2 @ Rule { name: name_rule_2, do_continue: true, active: true, .. }] =>
            {
                assert_eq!("log_internal_snmp_traps", name_rule_1);
                assert_eq!("log_external_snmp_traps", name_rule_2);
                (rule1, rule2)
            }
            result => panic!("{:#?}", result),
        };

        let operators = match rule1 {
            Rule {
                constraint: Constraint { where_operator: Some(Operator::And { operators }), with },
                actions,
                ..
            } => {
                assert!(with.is_empty());
                assert_eq!(1, actions.len());
                operators
            }
            result => panic!("{:#?}", result),
        };

        match operators.as_slice() {
            [Operator::Equals { first, second }, Operator::Regex { .. }] => {
                assert_eq!(&Value::String("${event.type}".to_string()), first);
                assert_eq!(&Value::String("snmptrapd".to_string()), second);
            }
            result => panic!("{:#?}", result),
        }

        match rule2 {
            Rule {
                constraint: Constraint { where_operator: Some(Operator::And { .. }), with },
                ..
            } => {
                assert!(with.is_empty());
            }
            result => panic!("{:#?}", result),
        };
    }

    #[tokio::test]
    async fn should_parse_ruleset_with_rules_from_directory() {
        let path = String::from(TEST_CONFIG_DIR) + "tenant_a/snmp_logger/";
        let config = read_ruleset_from_dir(Path::new(&path)).await.unwrap();

        let rules = match config {
            MatcherConfig::Ruleset { name, rules } => {
                assert_eq!("snmp_logger", name);
                rules
            }
            result => panic!("{:#?}", result),
        };

        match rules.as_slice() {
            [Rule { name: name_rule_1, do_continue: true, active: true, .. }, Rule { name: name_rule_2, do_continue: true, active: true, .. }] =>
            {
                assert_eq!("log_internal_snmp_traps", name_rule_1);
                assert_eq!("log_external_snmp_traps", name_rule_2);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_parse_empty_filter_from_dir() {
        let path = String::from(TEST_CONFIG_DIR) + "empty_filter/";
        let config = read_filter_from_dir(Path::new(&path)).await.unwrap();

        match config {
            MatcherConfig::Filter { name, nodes, .. } => {
                assert_eq!("empty_filter", name);
                assert!(nodes.is_empty());
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn parse_filter_with_children_from_dir() {
        let path = String::from(TEST_CONFIG_DIR) + "master/";
        let config = read_filter_from_dir(Path::new(&path)).await.unwrap();

        dbg!(&config);

        let nodes = match config {
            MatcherConfig::Filter { name, filter, nodes } => {
                assert_eq!("master", name);
                assert!(matches!(
                    filter,
                    Filter { filter: Defaultable::Value(Operator::And { .. }), .. }
                ));
                nodes
            }
            result => panic!("{:#?}", result),
        };

        match nodes.as_slice() {
            [MatcherConfig::Filter { name, filter, nodes }] => {
                assert_eq!("master_emails", name);
                assert!(matches!(
                    filter,
                    Filter { filter: Defaultable::Value(Operator::And { .. }), .. }
                ));
                assert_eq!(1, nodes.len());
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_read_entire_config_from_dir() {
        let config = read_config_from_root_dir(Path::new(&TEST_CONFIG_DIR)).await.unwrap();
        let nodes = match config {
            MatcherConfig::Filter { name, filter, nodes } => {
                assert_eq!("root", name);
                assert!(matches!(filter, Filter { filter: Defaultable::Default {}, .. }));
                nodes
            }
            result => panic!("{:#?}", result),
        };

        match nodes.as_slice() {
            [MatcherConfig::Filter { name: name_filter_1, .. }, MatcherConfig::Filter { name: name_filter_2, .. }, MatcherConfig::Filter { name: name_filter_3, .. }] =>
            {
                assert_eq!("empty_filter", name_filter_1);
                assert_eq!("master", name_filter_2);
                assert_eq!("tenant_a", name_filter_3);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_fail_on_missing_field() {
        let test_file_path = String::from(TEST_BROKEN_CONFIG_DIR) + "filter_missing_field.json";
        let error = parse_config_from_file::<MatcherConfigFilter>(Path::new(&test_file_path))
            .await
            .unwrap_err();

        match error {
            MatcherConfigError::DeserializationError {
                file,
                error: DeserializationError::MissingField { path, field },
            } => {
                assert_eq!(&test_file_path, &format!("{}", file.display()));
                assert_eq!(".", &path);
                assert_eq!("name", &field);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_fail_on_unknown_field() {
        let test_file_path = String::from(TEST_BROKEN_CONFIG_DIR) + "filter_unknown_field.json";
        let error = parse_config_from_file::<MatcherConfigFilter>(Path::new(&test_file_path))
            .await
            .unwrap_err();

        match error {
            MatcherConfigError::DeserializationError {
                file,
                error: DeserializationError::UnknownField { path, field },
            } => {
                assert_eq!(&test_file_path, &format!("{}", file.display()));
                assert_eq!(".", path);
                assert_eq!("pippo", field);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_fail_on_wrong_data() {
        let test_file_path = String::from(TEST_BROKEN_CONFIG_DIR) + "filter_wrong_data.json";
        let error = parse_config_from_file::<MatcherConfigFilter>(Path::new(&test_file_path))
            .await
            .unwrap_err();

        match error {
            MatcherConfigError::DeserializationError {
                file,
                error: DeserializationError::TypeError { path, expected_type, actual_type },
            } => {
                assert_eq!(&test_file_path, &format!("{}", file.display()));
                assert_eq!("name", path);
                assert_eq!("string", expected_type);
                assert_eq!("integer", actual_type);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_fail_on_wrong_type() {
        let test_file_path = String::from(TEST_BROKEN_CONFIG_DIR) + "filter_wrong_type.json";
        let error = parse_config_from_file::<MatcherConfigFilter>(Path::new(&test_file_path))
            .await
            .unwrap_err();

        match error {
            MatcherConfigError::DeserializationError {
                file,
                error:
                    DeserializationError::InvalidField {
                        path,
                        found,
                        found_type,
                        expected,
                        expected_type,
                    },
            } => {
                assert_eq!(&test_file_path, &format!("{}", file.display()));
                assert_eq!("type", path);
                assert_eq!("string", found_type);
                assert_eq!("\"ruleset\"", found);
                assert_eq!("string", expected_type);
                assert_eq!("\"filter\"", expected);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_fail_on_missing_node_file() {
        let test_file_path = String::from(TEST_BROKEN_CONFIG_DIR) + "dir_missing_node_file/";
        let error = read_node_from_dir(Path::new(&test_file_path)).await.unwrap_err();

        match error {
            MatcherConfigError::UnknownNodeDir { path } => {
                assert_eq!(&test_file_path, &format!("{}", path.display()));
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_fail_on_wrong_version() {
        let test_file_path = String::from(TEST_BROKEN_CONFIG_DIR) + "config_wrong_version/";
        let error = read_config_from_root_dir(Path::new(&test_file_path)).await.unwrap_err();

        match error {
            MatcherConfigError::DeserializationError {
                file,
                error: DeserializationError::InvalidField { path, .. },
            } => {
                let version_file = test_file_path.clone() + "version.json";
                assert_eq!(&version_file, &format!("{}", file.display()));
                assert_eq!("version", path);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_fail_on_extra_file() {
        let test_file_path = String::from(TEST_BROKEN_CONFIG_DIR) + "filter_extra_file/";
        let error = read_filter_from_dir(Path::new(&test_file_path)).await.unwrap_err();

        match dbg!(error) {
            MatcherConfigError::UnexpectedFile { path, config_type } => {
                let path = format!("{}", path.display());
                assert!(path.starts_with(&test_file_path));
                assert_eq!(ConfigType::Filter, config_type);
            }
            result => panic!("{:#?}", result),
        }
    }
}
