use crate::config::filter::Filter;
use crate::config::rule::{Rule, Operator};
use crate::config::{MatcherConfig, MatcherConfigManager};
use crate::error::MatcherError;
use log::*;
use std::ffi::OsStr;
use std::fs;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

pub const ROOT_NODE_NAME: &str = "root";

pub struct FsMatcherConfigManager {
    root_path: String,
}

impl FsMatcherConfigManager {
    pub fn new<P: Into<String>>(root_path: P) -> FsMatcherConfigManager {
        FsMatcherConfigManager { root_path: root_path.into() }
    }
}

#[derive(Debug, PartialEq)]
pub enum DirType {
    Filter,
    Ruleset,
}

impl MatcherConfigManager for FsMatcherConfigManager {
    fn read(&self) -> Result<MatcherConfig, MatcherError> {
        FsMatcherConfigManager::read_from_root_dir(&self.root_path)
    }
}

impl FsMatcherConfigManager {
    fn read_from_root_dir<P: AsRef<Path>>(dir: P) -> Result<MatcherConfig, MatcherError> {
        FsMatcherConfigManager::read_from_dir(ROOT_NODE_NAME, dir)
    }

    fn read_from_dir<P: AsRef<Path>>(
        node_name: &str,
        dir: P,
    ) -> Result<MatcherConfig, MatcherError> {
        match FsMatcherConfigManager::detect_dir_type(dir.as_ref())? {
            DirType::Filter => {
                FsMatcherConfigManager::read_filter_from_dir(node_name, dir.as_ref())
            }
            DirType::Ruleset => {
                FsMatcherConfigManager::read_ruleset_from_dir(node_name, dir.as_ref())
            }
        }
    }

    // Returns whether the directory contains a filter. Otherwise it contains rules.
    // These logic is used to determine the folder content:
    // - It contains a filter if there max one json file AND at least one subdirectory. The result is true.
    // - It contains a rule set if there are no subdirectories. The result is false.
    // - It returns an error in every other case.
    fn detect_dir_type<P: AsRef<Path>>(dir: P) -> Result<DirType, MatcherError> {
        let paths = FsMatcherConfigManager::read_dir_entries(dir.as_ref())?;

        let mut subdirectories_count = 0;
        let mut json_files_count = 0;

        for entry in paths {
            let path = entry.path();

            if path.is_dir() {
                subdirectories_count += 1;
            } else {
                let filename = FsMatcherConfigManager::filename(&path)?;
                if filename.ends_with(".json") {
                    json_files_count += 1;
                }
            }
        }
        debug!(
            "Path {} contains {} file(s) and {} directories",
            dir.as_ref().display(),
            json_files_count,
            subdirectories_count
        );

        if subdirectories_count > 0 {
            if json_files_count <= 1 {
                return Ok(DirType::Filter);
            }
            return Err(MatcherError::ConfigurationError {
                message: format!(
                    r#"Path {} contains {} file(s) and {} directories. Expected:\n
                 for a valid filter: max one json file and at least one directory;\n
                 for a valid rule set: zero or more json files and no directories."#,
                    dir.as_ref().display(),
                    json_files_count,
                    subdirectories_count
                ),
            });
        }
        Ok(DirType::Ruleset)
    }

    fn read_ruleset_from_dir<P: AsRef<Path>>(
        node_name: &str,
        dir: P,
    ) -> Result<MatcherConfig, MatcherError> {
        let paths = FsMatcherConfigManager::read_dir_entries(dir.as_ref())?;

        let mut rules = vec![];

        for entry in paths {
            let path = entry.path();

            let filename = FsMatcherConfigManager::filename(&path)?;
            let extension = ".json";

            if !filename.ends_with(extension) {
                warn!("Configuration file [{}] is ignored.", path.display());
                continue;
            }

            debug!("Loading rule from file: [{}]", path.display());
            let rule_body =
                fs::read_to_string(&path).map_err(|e| MatcherError::ConfigurationError {
                    message: format!("Unable to open the file [{}]. Err: {}", path.display(), e),
                })?;

            trace!("Rule body: \n{}", rule_body);
            let mut rule =
                Rule::from_json(&rule_body).map_err(|e| MatcherError::ConfigurationError {
                    message: format!(
                        "Error building Rule from file [{}]. Err: {}",
                        path.display(),
                        e
                    ),
                })?;
            rule.name = FsMatcherConfigManager::rule_name_from_filename(
                &FsMatcherConfigManager::truncate(filename, extension.len()),
            )?
            .to_owned();
            rules.push(rule);
        }

        info!("Loaded {} rule(s) from [{}]", rules.len(), dir.as_ref().display());

        Ok(MatcherConfig::Ruleset { name: node_name.to_owned(), rules })
    }

    fn read_filter_from_dir<P: AsRef<Path>>(
        node_name: &str,
        dir: P,
    ) -> Result<MatcherConfig, MatcherError> {
        let paths = FsMatcherConfigManager::read_dir_entries(dir.as_ref())?;

        let mut nodes = vec![];
        let mut filters = vec![];

        for entry in paths {
            let path = entry.path();

            let filename = FsMatcherConfigManager::filename(&path)?;

            if path.is_dir() {
                // A filter contains a set of subdirectories that can recursively contain other filters
                // or rule sets. We call FsMatcherConfigManager::read_from_dir recursively to build this nested tree
                // of inner structures.
                nodes.push(FsMatcherConfigManager::read_from_dir(filename, path.as_path())?);
                continue;
            }

            let extension = ".json";
            if !filename.ends_with(extension) {
                info!("Configuration file [{}] is ignored.", path.display());
                continue;
            }

            info!("Loading filter from file: [{}]", path.display());
            let filter_body =
                fs::read_to_string(&path).map_err(|e| MatcherError::ConfigurationError {
                    message: format!("Unable to open the file [{}]. Err: {}", path.display(), e),
                })?;

            trace!("Filter [{}] body: \n{}", filename, filter_body);
            let filter =
                Filter::from_json(&filter_body).map_err(|e| MatcherError::ConfigurationError {
                    message: format!(
                        "Error building Filter from file [{}]. Err: {}",
                        path.display(),
                        e
                    ),
                })?;
            filters.push(filter);
        }

        let name = node_name.to_owned();

        if filters.is_empty() && !nodes.is_empty() {
            let filter = Filter {
                active: true,
                description: "An implicit filter that allows all events".to_owned(),
                filter: Operator::Always,
            };
            return Ok(MatcherConfig::Filter { name, filter, nodes });
        }

        if filters.len() == 1 && !nodes.is_empty() {
            let filter = filters.remove(0);
            return Ok(MatcherConfig::Filter { name, filter, nodes });
        }

        Err(MatcherError::ConfigurationError {
            message: format!("Config path [{}] contains {} json files and {} subdirectories. Expected exactly one json filter file and at least one subdirectory.",
                             dir.as_ref().display(), filters.len(), nodes.len()),
        })
    }

    fn read_dir_entries<P: AsRef<Path>>(dir: P) -> Result<Vec<DirEntry>, MatcherError> {
        let mut paths: Vec<_> =
            fs::read_dir(dir.as_ref()).and_then(Iterator::collect).map_err(|e| {
                MatcherError::ConfigurationError {
                    message: format!(
                        "Error reading from config path [{}]: {}",
                        dir.as_ref().display(),
                        e
                    ),
                }
            })?;
        // Sort by filename
        paths.sort_by_key(DirEntry::path);
        Ok(paths)
    }

    fn truncate(name: &str, truncate: usize) -> String {
        let mut name = name.to_owned();
        name.truncate(name.len() - truncate);
        name
    }

    fn filename(path: &PathBuf) -> Result<&str, MatcherError> {
        path.file_name().and_then(OsStr::to_str).ok_or_else(|| MatcherError::ConfigurationError {
            message: format!("Error processing path name: [{}]", path.display()),
        })
    }

    fn rule_name_from_filename(filename: &str) -> Result<&str, MatcherError> {
        let split_char = '_';
        let mut splitter = filename.splitn(2, split_char);
        let mut result = "";
        for _ in 0..2 {
            result = splitter.next().ok_or_else(|| MatcherError::ConfigurationError {
                message: format!(
                    "Error extracting rule name from filename [{}]. The filename must contain at least one '{}' char to separate the first part of the filename from the rule name.",
                    filename, split_char,
                ),
            })?
        }
        Ok(result)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::fs;

    #[test]
    fn should_read_rules_from_folder_sorting_by_filename() {
        let path = "./test_resources/rules";
        let config = FsMatcherConfigManager::new(path).read().unwrap();

        match config {
            MatcherConfig::Ruleset { name, rules } => {
                assert_eq!("root", name);

                assert_eq!(4, rules.len());

                assert_eq!("all_emails_and_syslogs", rules.get(0).unwrap().name);
                assert_eq!("rule_without_where", rules.get(1).unwrap().name);
                assert_eq!("map_in_action_payload", rules.get(2).unwrap().name);
                assert_eq!("cmp_operators", rules.get(3).unwrap().name);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_read_rules_from_empty_folder() {
        let path = "./test_resources/config_empty";
        let config = FsMatcherConfigManager::new(path).read().unwrap();

        match config {
            MatcherConfig::Ruleset { name, rules } => {
                assert_eq!("root", name);
                assert_eq!(0, rules.len());
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_read_filter_from_folder() {
        let path = "./test_resources/config_01";
        let config = FsMatcherConfigManager::read_from_dir("custom_name", path).unwrap();

        assert!(is_filter(&config, "custom_name", 1));
    }

    fn is_filter(config: &MatcherConfig, name: &str, nodes_num: usize) -> bool {
        match config {
            MatcherConfig::Filter { name: filter_name, filter: _filter, nodes } => {
                name.eq(filter_name) && nodes.len() == nodes_num
            }
            _ => false,
        }
    }

    fn is_ruleset(config: &MatcherConfig, name: &str, rule_names: &[&str]) -> bool {
        match config {
            MatcherConfig::Ruleset { name: ruleset_name, rules } => {
                let mut result = name.eq(ruleset_name) && rules.len() == rule_names.len();
                for i in 0..rule_names.len() {
                    result = result && rules[i].name.eq(rule_names[i]);
                }
                result
            }
            _ => false,
        }
    }

    #[test]
    fn should_read_from_folder_and_return_error_if_not_a_rule() {
        let path = "./test_resources/config_02";
        let config = FsMatcherConfigManager::read_from_root_dir(path);

        assert!(config.is_err());
    }

    #[test]
    fn should_read_filter_from_folder_with_many_subfolders() {
        let path = "./test_resources/config_03";
        let config = FsMatcherConfigManager::read_from_dir("emails", path).unwrap();

        assert!(is_filter(&config, "emails", 2));
    }

    #[test]
    fn should_read_config_from_folder_recursively() {
        let path = "./test_resources/config_04";
        let config = FsMatcherConfigManager::read_from_root_dir(path).unwrap();

        println!("{:?}", config);

        assert!(is_filter(&config, "root", 2));

        match config {
            MatcherConfig::Filter { name: _, filter: _, nodes } => {
                assert!(is_filter(get_config_by_name("node1", &nodes).unwrap(), "node1", 1));
                assert!(is_ruleset(
                    get_config_by_name("node2", &nodes).unwrap(),
                    "node2",
                    &vec!["rule1"]
                ));

                match get_config_by_name("node1", &nodes).unwrap() {
                    MatcherConfig::Filter { name: _, filter: _, nodes: inner_nodes } => {
                        assert!(is_ruleset(
                            get_config_by_name("inner_node1", &inner_nodes).unwrap(),
                            "inner_node1",
                            &vec!["rule2", "rule3"]
                        ));
                    }
                    _ => assert!(false),
                }
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_create_implicit_filter_recursively() {
        let path = "./test_resources/config_implicit_filter";
        let config = FsMatcherConfigManager::read_from_dir("implicit", path).unwrap();
        println!("{:?}", config);

        assert!(is_filter(&config, "implicit", 2));

        match config {
            MatcherConfig::Filter { name: _name, filter: root_filter, nodes } => {
                assert_eq!(Operator::Always, root_filter.filter);
                assert!(is_filter(get_config_by_name("node1", &nodes).unwrap(), "node1", 1));
                assert!(is_ruleset(
                    get_config_by_name("node2", &nodes).unwrap(),
                    "node2",
                    &vec!["rule1"]
                ));

                match get_config_by_name("node1", &nodes).unwrap() {
                    MatcherConfig::Filter {
                        name: _name,
                        filter: inner_filter,
                        nodes: inner_nodes,
                    } => {
                        assert_eq!(Operator::Always, inner_filter.filter);
                        assert!(is_ruleset(
                            get_config_by_name("inner_node1", &inner_nodes).unwrap(),
                            "inner_node1",
                            &vec!["rule2"]
                        ));
                    }
                    _ => assert!(false),
                }
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_return_dir_type_filter_if_one_file_and_one_subdir() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        fs::create_dir_all(&format!("{}/subdir", dir)).unwrap();
        fs::File::create(&format!("{}/file.json", dir)).unwrap();

        // Act
        let result = FsMatcherConfigManager::detect_dir_type(&dir);

        // Assert
        assert_eq!(Ok(DirType::Filter), result);
    }

    #[test]
    fn should_return_dir_type_rules_if_one_file_and_no_subdir() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        fs::File::create(&format!("{}/file.json", dir)).unwrap();

        // Act
        let result = FsMatcherConfigManager::detect_dir_type(&dir);

        // Assert
        assert_eq!(Ok(DirType::Ruleset), result);
    }

    #[test]
    fn should_return_dir_type_rules_if_many_files_and_no_subdir() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        fs::File::create(&format!("{}/file_01.json", dir)).unwrap();
        fs::File::create(&format!("{}/file_02.json", dir)).unwrap();
        fs::File::create(&format!("{}/file_03.json", dir)).unwrap();

        // Act
        let result = FsMatcherConfigManager::detect_dir_type(&dir);

        // Assert
        assert_eq!(Ok(DirType::Ruleset), result);
    }

    #[test]
    fn should_return_dir_type_filter_if_no_files_but_subdirs() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        fs::create_dir_all(&format!("{}/subdir1", dir)).unwrap();
        fs::create_dir_all(&format!("{}/subdir2", dir)).unwrap();

        // Act
        let result = FsMatcherConfigManager::detect_dir_type(&dir);

        // Assert
        assert_eq!(Ok(DirType::Filter), result);
    }

    #[test]
    fn is_filter_dir_should_return_error_if_many_files_and_subdirs() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        fs::create_dir_all(&format!("{}/subdir1", dir)).unwrap();
        fs::create_dir_all(&format!("{}/subdir2", dir)).unwrap();
        fs::File::create(&format!("{}/file1.json", dir)).unwrap();
        fs::File::create(&format!("{}/file2.json", dir)).unwrap();

        // Act
        let result = FsMatcherConfigManager::detect_dir_type(&dir);

        // Assert
        assert!(result.is_err());
        match result {
            Err(e) => match e {
                MatcherError::ConfigurationError { message } => assert!(message.contains(
                    &format!("Path {} contains {} file(s) and {} directories.", dir, 2, 2)
                )),
                _ => assert!(false),
            },
            _ => assert!(false),
        }
    }

    #[test]
    fn should_return_rule_name_from_file_name() {
        // not valid names
        assert!(FsMatcherConfigManager::rule_name_from_filename("").is_err());
        assert!(FsMatcherConfigManager::rule_name_from_filename("1245345").is_err());
        assert!(FsMatcherConfigManager::rule_name_from_filename("asfg.rulename").is_err());

        // valid names
        assert_eq!(
            "rulename",
            FsMatcherConfigManager::rule_name_from_filename("_rulename").unwrap()
        );
        assert_eq!(
            "rulename",
            FsMatcherConfigManager::rule_name_from_filename("12343_rulename").unwrap()
        );
        assert_eq!(
            "rule_name_1",
            FsMatcherConfigManager::rule_name_from_filename("ascfb5.46_rule_name_1").unwrap()
        );
        assert_eq!(
            "__rule_name_1",
            FsMatcherConfigManager::rule_name_from_filename("ascfb5.46___rule_name_1").unwrap()
        );
        assert_eq!(
            "rule_name_1__._",
            FsMatcherConfigManager::rule_name_from_filename("ascfb5.46_rule_name_1__._").unwrap()
        );
    }

    fn get_config_name(config: &MatcherConfig) -> &str {
        match config {
            MatcherConfig::Filter { name, .. } => name,
            MatcherConfig::Ruleset { name, .. } => name,
        }
    }

    fn get_config_by_name<'a>(name: &str, nodes: &'a [MatcherConfig]) -> Option<&'a MatcherConfig> {
        for node in nodes {
            if get_config_name(node).eq(name) {
                return Some(node);
            }
        }
        None
    }
}
