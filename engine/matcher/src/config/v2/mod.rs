mod editor;
mod error;

use crate::config::filter::Filter;
use crate::config::rule::Rule;
pub use crate::config::v2::error::MatcherConfigError;
use crate::config::{Defaultable, MatcherConfig, MatcherConfigReader};
use crate::error::MatcherError;
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use log::{debug, error, info, trace, warn};
use monostate::MustBe;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::error::Category;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use tokio::fs::DirEntry;

pub struct FsMatcherConfigManagerV2 {
    root_path: PathBuf,
    drafts_path: PathBuf,
}

impl FsMatcherConfigManagerV2 {
    pub fn new<P1: Into<PathBuf>, P2: Into<PathBuf>>(
        root_path: P1,
        drafts_path: P2,
    ) -> FsMatcherConfigManagerV2 {
        FsMatcherConfigManagerV2 { root_path: root_path.into(), drafts_path: drafts_path.into() }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ConfigType {
    Draft,
    Root,
    Filter,
    Ruleset,
}

impl Display for ConfigType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigType::Draft => f.write_str("draft"),
            ConfigType::Root => f.write_str("root"),
            ConfigType::Filter => f.write_str("filter"),
            ConfigType::Ruleset => f.write_str("ruleset"),
        }
    }
}

pub trait ConfigNodeDir {
    fn config_type() -> ConfigType;
}

impl ConfigType {
    pub fn filename(&self) -> &'static str {
        match self {
            ConfigType::Root => "version.json",
            ConfigType::Filter => "filter.json",
            ConfigType::Ruleset => "ruleset.json",
            ConfigType::Draft => "data.json",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatcherConfigFilter {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    node_type: MustBe!("filter"),
    name: String,
    #[serde(flatten)]
    filter: Filter,
}

impl ConfigNodeDir for MatcherConfigFilter {
    fn config_type() -> ConfigType {
        ConfigType::Filter
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatcherConfigRuleset {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    node_type: MustBe!("ruleset"),
    name: String,
}

impl ConfigNodeDir for MatcherConfigRuleset {
    fn config_type() -> ConfigType {
        ConfigType::Ruleset
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "version")]
pub enum Version {
    V1,
    #[serde(rename = "2.0")]
    #[default]
    V2,
}

impl ConfigNodeDir for Version {
    fn config_type() -> ConfigType {
        ConfigType::Root
    }
}

// ToDo: Improve the error handling in NEPROD-1658
#[async_trait::async_trait(?Send)]
impl MatcherConfigReader for FsMatcherConfigManagerV2 {
    async fn get_config(&self) -> Result<MatcherConfig, MatcherError> {
        Ok(read_config_from_root_dir(&self.root_path).await?)
    }
}

pub async fn get_config_version(path: &Path) -> Result<Version, MatcherConfigError> {
    match parse_node_config_from_file::<Version>(path).await {
        Ok(version) => Ok(version),
        // Fallback as the file version.json did not exist in version 1.
        Err(MatcherConfigError::FileNotFound { .. }) => Ok(Version::V1),
        Err(error) => Err(error),
    }
}

async fn read_config_from_root_dir(root_dir: &Path) -> Result<MatcherConfig, MatcherConfigError> {
    info!("Reading tornado processing tree configuration from {}", root_dir.display());

    let _: Version = parse_node_config_from_file(root_dir).await?;
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
    debug!("Found {} entries in the directory {}", dir_entries.len(), dir.display());

    let mut processing_nodes_futures = FuturesOrdered::new();
    for dir_entry in dir_entries {
        trace!("Parsing directory entry {}", dir_entry.path().display());

        let file_type = match dir_entry.file_type().await {
            Ok(file_type) => file_type,
            Err(error) => {
                error!(
                    "Could not load filetype for directory directory entry {}",
                    dir_entry.path().display()
                );
                return Err(MatcherConfigError::DirIoError { path: PathBuf::from(dir), error });
            }
        };

        if file_type.is_file() {
            if dir_entry.file_name() == config_type.filename() {
                debug!(
                    "Skipping file {} because it is a node configuration.",
                    config_type.filename()
                );
                continue;
            }

            if dir_entry.path().extension() != Some(OsStr::new("json")) {
                info!("Ignoring file [{}] as it is not a config file.", dir_entry.path().display());
                continue;
            }

            error!("Unexpected file {}", dir_entry.path().display());
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

    let mut processing_nodes: Vec<FileEntry<MatcherConfig>> = vec![];
    while let Some(result) = processing_nodes_futures.next().await {
        let config = result?;
        let duplicate = processing_nodes
            .iter()
            .find(|entry| entry.content.get_name() == config.content.get_name());

        if let Some(duplicate) = duplicate {
            return Err(MatcherConfigError::DuplicateName {
                name: config.content.get_name().to_string(),
                previous: duplicate.path.clone(),
                next: config.path,
            });
        }

        processing_nodes.push(config);
    }

    Ok(processing_nodes.into_iter().map(FileEntry::into_inner).collect())
}

async fn read_node_from_dir<T: AsRef<Path>>(
    dir: T,
) -> Result<FileEntry<MatcherConfig>, MatcherConfigError> {
    let dir = dir.as_ref();
    match read_filter_from_dir(dir).await {
        Ok(config) => return Ok(FileEntry { path: dir.to_path_buf(), content: config }),
        Err(MatcherConfigError::FileNotFound { .. }) => {
            trace!("Directory {} seems not to be a filter node.", dir.display())
        }
        Err(error) => return Err(error),
    }

    match read_ruleset_from_dir(dir).await {
        Ok(config) => Ok(FileEntry { path: dir.to_path_buf(), content: config }),
        Err(MatcherConfigError::FileNotFound { .. }) => {
            trace!("Directory {} seems not to be a ruleset node.", dir.display());
            Err(MatcherConfigError::UnknownNodeDir { path: dir.to_path_buf() })
        }
        Err(error) => Err(error),
    }
}

async fn read_filter_from_dir(dir: &Path) -> Result<MatcherConfig, MatcherConfigError> {
    trace!("Reading filer node config file from disk.");
    let node: MatcherConfigFilter = parse_node_config_from_file(dir).await?;
    trace!("Reading filer node child nodes from disk.");
    let child_nodes = read_child_nodes_from_dir(dir, MatcherConfigFilter::config_type()).await?;

    Ok(MatcherConfig::Filter { name: node.name, filter: node.filter, nodes: child_nodes })
}

async fn read_ruleset_from_dir(dir: &Path) -> Result<MatcherConfig, MatcherConfigError> {
    let rules_dir_path = {
        let mut path = PathBuf::from(dir);
        path.push("rules");
        path
    };

    trace!("Reading ruleset node config file from disk.");
    let ruleset: MatcherConfigRuleset = parse_node_config_from_file(dir).await?;
    trace!("Reading ruleset node rules from disk.");
    let rules = read_rules_from_dir(&rules_dir_path).await?;

    Ok(MatcherConfig::Ruleset { name: ruleset.name, rules })
}

async fn read_rules_from_dir(dir: &Path) -> Result<Vec<Rule>, MatcherConfigError> {
    let dir_entries = gather_dir_entries(dir).await?;

    let mut rules: Vec<FileEntry<Rule>> = vec![];
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

        let rule: Rule = parse_from_file(&dir_entry.path()).await?;
        let duplicate = rules.iter().find(|entry| entry.content.name == rule.name);

        if let Some(duplicate) = duplicate {
            return Err(MatcherConfigError::DuplicateName {
                name: rule.name,
                previous: duplicate.path.clone(),
                next: dir_entry.path(),
            });
        }

        rules.push(FileEntry { path: dir_entry.path(), content: rule })
    }

    Ok(rules.into_iter().map(FileEntry::into_inner).collect())
}

async fn parse_node_config_from_file<Data: DeserializeOwned + ConfigNodeDir>(
    dir: &Path,
) -> Result<Data, MatcherConfigError> {
    let config_file_path = {
        let mut path = PathBuf::from(dir);
        path.push(Data::config_type().filename());
        path
    };

    trace!(
        "Try reading data of type {} from file {}",
        std::any::type_name::<Data>(),
        config_file_path.display()
    );

    parse_from_file(&config_file_path).await
}

async fn parse_from_file<Data: DeserializeOwned>(path: &Path) -> Result<Data, MatcherConfigError> {
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
        Err(error) => {
            error!("Could not parse config from file {}. {}", path.display(), error);
            match error.inner().classify() {
                Category::Io | Category::Eof => Err(MatcherConfigError::FormatError {
                    file: path.to_path_buf(),
                    error: error.into_inner(),
                }),
                Category::Syntax | Category::Data => {
                    Err(MatcherConfigError::DeserializationError {
                        file: path.to_path_buf(),
                        object_path: error.path().to_string(),
                        error: error.into_inner(),
                    })
                }
            }
        }
    }
}

pub async fn gather_dir_entries(dir: &Path) -> Result<Vec<DirEntry>, MatcherConfigError> {
    let mut root_dir_iter = match tokio::fs::read_dir(dir).await {
        Ok(root_dir_iter) => root_dir_iter,
        Err(error) => {
            return Err(MatcherConfigError::DirIoError { path: PathBuf::from(dir), error })
        }
    };

    debug!("Collecting entries of directory {}", dir.display());

    let mut dir_entries = vec![];
    loop {
        match root_dir_iter.next_entry().await {
            Ok(Some(entry)) => {
                trace!("Found entry {}", entry.path().display());
                dir_entries.push(entry)
            }
            Ok(None) => break,
            Err(error) => {
                return Err(MatcherConfigError::DirIoError { path: PathBuf::from(dir), error })
            }
        };
    }

    dir_entries.sort_by_key(DirEntry::path);
    Ok(dir_entries)
}

#[derive(Debug)]
pub struct FileEntry<T> {
    path: PathBuf,
    content: T,
}

impl<T> FileEntry<T> {
    fn into_inner(self) -> T {
        self.content
    }
}

#[cfg(test)]
mod tests {
    use crate::config::filter::Filter;
    use crate::config::rule::{ConfigAction, Constraint, Operator, Rule};
    use crate::config::v2::{
        parse_from_file, read_config_from_root_dir, read_filter_from_dir, read_node_from_dir,
        read_rules_from_dir, read_ruleset_from_dir, ConfigType, MatcherConfigError,
        MatcherConfigFilter, MatcherConfigRuleset,
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
        let config: MatcherConfigFilter = parse_from_file(Path::new(&path)).await.unwrap();

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
        let config: MatcherConfigRuleset = parse_from_file(Path::new(&path)).await.unwrap();

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
        let config: Rule = parse_from_file(Path::new(&path)).await.unwrap();

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
        let error =
            parse_from_file::<MatcherConfigFilter>(Path::new(&test_file_path)).await.unwrap_err();

        match error {
            MatcherConfigError::DeserializationError { file, object_path, .. } => {
                assert_eq!(&test_file_path, &format!("{}", file.display()));
                assert_eq!(".", &object_path);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_fail_on_unknown_field() {
        let test_file_path = String::from(TEST_BROKEN_CONFIG_DIR) + "filter_unknown_field.json";
        let error =
            parse_from_file::<MatcherConfigFilter>(Path::new(&test_file_path)).await.unwrap_err();

        match error {
            MatcherConfigError::DeserializationError { file, object_path, .. } => {
                assert_eq!(&test_file_path, &format!("{}", file.display()));
                assert_eq!(".", object_path);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_fail_on_wrong_data() {
        let test_file_path = String::from(TEST_BROKEN_CONFIG_DIR) + "filter_wrong_data.json";
        let error =
            parse_from_file::<MatcherConfigFilter>(Path::new(&test_file_path)).await.unwrap_err();

        match error {
            MatcherConfigError::DeserializationError { file, object_path, .. } => {
                assert_eq!(&test_file_path, &format!("{}", file.display()));
                assert_eq!("name", object_path);
            }
            result => panic!("{:#?}", result),
        }
    }

    #[tokio::test]
    async fn should_fail_on_wrong_type() {
        let test_file_path = String::from(TEST_BROKEN_CONFIG_DIR) + "filter_wrong_type.json";
        let error =
            parse_from_file::<MatcherConfigFilter>(Path::new(&test_file_path)).await.unwrap_err();

        match error {
            MatcherConfigError::DeserializationError { file, object_path, .. } => {
                assert_eq!(&test_file_path, &format!("{}", file.display()));
                assert_eq!("type", object_path);
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
            MatcherConfigError::DeserializationError { file, object_path, .. } => {
                let version_file = test_file_path.clone() + "version.json";
                assert_eq!(&version_file, &format!("{}", file.display()));
                assert_eq!("version", object_path);
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
