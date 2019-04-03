use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::error::MatcherError;
use log::{debug, info, trace};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

pub mod filter;
pub mod rule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatcherConfig {
    Filter { filter: Filter, nodes: BTreeMap<String, MatcherConfig> },
    Rules { rules: Vec<Rule> },
}

#[derive(Debug, PartialEq)]
pub enum DirType {
    Filter,
    Rules,
}

impl MatcherConfig {
    pub fn read_from_dir<P: AsRef<Path>>(dir: P) -> Result<MatcherConfig, MatcherError> {
        match MatcherConfig::detect_dir_type(dir.as_ref())? {
            DirType::Filter => MatcherConfig::read_filter_from_dir(dir.as_ref()),
            DirType::Rules => MatcherConfig::read_rules_from_dir(dir.as_ref()),
        }
    }

    // Returns whether the directory contains a filter. Otherwise it contains rules.
    // These logic is used to determine the folder content:
    // - It contains a filter if there max one json file AND at least one subdirectory. The result is true.
    // - It contains a rule set if there are no subdirectories. The result is false.
    // - It returns an error in every other case.
    fn detect_dir_type<P: AsRef<Path>>(dir: P) -> Result<DirType, MatcherError> {
        let paths = MatcherConfig::read_dirs(dir.as_ref())?;

        let mut subdirectories_count = 0;
        let mut json_files_count = 0;

        for entry in paths {
            let path = entry.path();

            if path.is_dir() {
                subdirectories_count += 1;
            } else {
                let filename = MatcherConfig::filename(&path)?;
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
        Ok(DirType::Rules)
    }

    fn read_rules_from_dir<P: AsRef<Path>>(dir: P) -> Result<MatcherConfig, MatcherError> {
        let mut paths = MatcherConfig::read_dirs(dir.as_ref())?;

        // Sort by filename
        paths.sort_by_key(|dir| dir.path());

        let mut rules = vec![];

        for entry in paths {
            let path = entry.path();

            let filename = MatcherConfig::filename(&path)?;
            let extension = ".json";

            if !filename.ends_with(extension) {
                info!("Configuration file [{}] is ignored.", path.display());
                continue;
            }

            info!("Loading rule from file: [{}]", path.display());
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
            rule.name = MatcherConfig::rule_name_from_filename(&MatcherConfig::truncate(
                filename,
                extension.len(),
            ))?
            .to_owned();
            rules.push(rule);
        }

        info!("Loaded {} rule(s) from [{}]", rules.len(), dir.as_ref().display());

        Ok(MatcherConfig::Rules { rules })
    }

    fn read_filter_from_dir<P: AsRef<Path>>(dir: P) -> Result<MatcherConfig, MatcherError> {
        let mut paths = MatcherConfig::read_dirs(dir.as_ref())?;

        // Sort by filename
        paths.sort_by_key(|dir| dir.path());

        let mut nodes = BTreeMap::new();
        let mut filters = vec![];

        for entry in paths {
            let path = entry.path();

            let filename = MatcherConfig::filename(&path)?;

            if path.is_dir() {
                // A filter contains a set of subdirectories that can recursively contain other filters
                // or rule sets. We call MatcherConfig::read_from_dir recursively to build this nested tree
                // of inner structures.
                nodes.insert(filename.to_owned(), MatcherConfig::read_from_dir(path.as_path())?);
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
            let mut filter =
                Filter::from_json(&filter_body).map_err(|e| MatcherError::ConfigurationError {
                    message: format!(
                        "Error building Filter from file [{}]. Err: {}",
                        path.display(),
                        e
                    ),
                })?;
            filter.name = MatcherConfig::truncate(filename, extension.len());
            filters.push(filter);
        }

        if filters.is_empty() && !nodes.is_empty() {
            let filter = Filter {
                active: true,
                name: "implicit_filter".to_owned(),
                description: "An implicit filter that allows all events".to_owned(),
                filter: None,
            };
            return Ok(MatcherConfig::Filter { filter, nodes });
        }

        if filters.len() == 1 && !nodes.is_empty() {
            let filter = filters.remove(0);
            return Ok(MatcherConfig::Filter { filter, nodes });
        }

        Err(MatcherError::ConfigurationError {
            message: format!("Config path [{}] contains {} json files and {} subdirectories. Expected exactly one json filter file and at least one subdirectory.",
                             dir.as_ref().display(), filters.len(), nodes.len()),
        })
    }

    fn read_dirs<P: AsRef<Path>>(dir: P) -> Result<Vec<DirEntry>, MatcherError> {
        fs::read_dir(dir.as_ref())
            .and_then(|entry_set| entry_set.collect::<Result<Vec<_>, _>>())
            .map_err(|e| MatcherError::ConfigurationError {
                message: format!(
                    "Error reading from config path [{}]: {}",
                    dir.as_ref().display(),
                    e
                ),
            })
    }

    fn truncate(name: &str, truncate: usize) -> String {
        let mut name = name.to_owned();
        name.truncate(name.len() - truncate);
        name
    }

    fn filename(path: &PathBuf) -> Result<&str, MatcherError> {
        path.file_name().and_then(|name| name.to_str()).ok_or_else(|| {
            MatcherError::ConfigurationError {
                message: format!("Error processing path name: [{}]", path.display()),
            }
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
        let config = MatcherConfig::read_from_dir(path).unwrap();

        match config {
            MatcherConfig::Rules { rules } => {
                assert_eq!(3, rules.len());

                assert_eq!("all_emails_and_syslogs", rules.get(0).unwrap().name);
                assert_eq!("rule_without_where", rules.get(1).unwrap().name);
                assert_eq!("map_in_action_payload", rules.get(2).unwrap().name);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_read_rules_from_empty_folder() {
        let path = "./test_resources/config_empty";
        let config = MatcherConfig::read_from_dir(path).unwrap();

        match config {
            MatcherConfig::Rules { rules } => {
                assert_eq!(0, rules.len());
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_read_filter_from_folder() {
        let path = "./test_resources/config_01";
        let config = MatcherConfig::read_from_dir(path).unwrap();

        assert!(is_filter(&config, "only_emails", 1));
    }

    fn is_filter(config: &MatcherConfig, name: &str, nodes_num: usize) -> bool {
        match config {
            MatcherConfig::Filter { filter, nodes } => {
                filter.name.eq(name) && nodes.len() == nodes_num
            }
            _ => false,
        }
    }

    fn is_ruleset(config: &MatcherConfig, rule_names: &[&str]) -> bool {
        match config {
            MatcherConfig::Rules { rules } => {
                let mut result = rules.len() == rule_names.len();
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
        let config = MatcherConfig::read_from_dir(path);

        assert!(config.is_err());
    }

    #[test]
    fn should_read_filter_from_folder_with_many_subfolders() {
        let path = "./test_resources/config_03";
        let config = MatcherConfig::read_from_dir(path).unwrap();

        assert!(is_filter(&config, "only_emails", 2));
    }

    #[test]
    fn should_read_config_from_folder_recursively() {
        let path = "./test_resources/config_04";
        let config = MatcherConfig::read_from_dir(path).unwrap();

        println!("{:?}", config);

        assert!(is_filter(&config, "filter1", 2));

        match config {
            MatcherConfig::Filter { filter: _, nodes } => {
                assert!(nodes.contains_key("node1"));
                assert!(nodes.contains_key("node2"));
                assert!(is_filter(&nodes["node1"], "filter2", 1));
                assert!(is_ruleset(&nodes["node2"], &vec!["rule1"]));

                match &nodes["node1"] {
                    MatcherConfig::Filter { filter: _, nodes: inner_nodes } => {
                        assert!(inner_nodes.contains_key("inner_node1"));
                        assert!(is_ruleset(&inner_nodes["inner_node1"], &vec!["rule2", "rule3"]));
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
        let config = MatcherConfig::read_from_dir(path).unwrap();
        println!("{:?}", config);

        assert!(is_filter(&config, "implicit_filter", 2));

        match config {
            MatcherConfig::Filter { filter: root_filter, nodes } => {
                assert!(root_filter.filter.is_none());
                assert!(nodes.contains_key("node1"));
                assert!(nodes.contains_key("node2"));
                assert!(is_filter(&nodes["node1"], "implicit_filter", 1));
                assert!(is_ruleset(&nodes["node2"], &vec!["rule1"]));

                match &nodes["node1"] {
                    MatcherConfig::Filter { filter: inner_filter, nodes: inner_nodes } => {
                        assert!(inner_filter.filter.is_none());
                        assert!(inner_nodes.contains_key("inner_node1"));
                        assert!(is_ruleset(&inner_nodes["inner_node1"], &vec!["rule2"]));
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
        let result = MatcherConfig::detect_dir_type(&dir);

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
        let result = MatcherConfig::detect_dir_type(&dir);

        // Assert
        assert_eq!(Ok(DirType::Rules), result);
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
        let result = MatcherConfig::detect_dir_type(&dir);

        // Assert
        assert_eq!(Ok(DirType::Rules), result);
    }

    #[test]
    fn should_return_dir_type_filter_if_no_files_but_subdirs() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        fs::create_dir_all(&format!("{}/subdir1", dir)).unwrap();
        fs::create_dir_all(&format!("{}/subdir2", dir)).unwrap();

        // Act
        let result = MatcherConfig::detect_dir_type(&dir);

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
        let result = MatcherConfig::detect_dir_type(&dir);

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
        assert!(MatcherConfig::rule_name_from_filename("").is_err());
        assert!(MatcherConfig::rule_name_from_filename("1245345").is_err());
        assert!(MatcherConfig::rule_name_from_filename("asfg.rulename").is_err());

        // valid names
        assert_eq!("rulename", MatcherConfig::rule_name_from_filename("_rulename").unwrap());
        assert_eq!("rulename", MatcherConfig::rule_name_from_filename("12343_rulename").unwrap());
        assert_eq!(
            "rule_name_1",
            MatcherConfig::rule_name_from_filename("ascfb5.46_rule_name_1").unwrap()
        );
        assert_eq!(
            "__rule_name_1",
            MatcherConfig::rule_name_from_filename("ascfb5.46___rule_name_1").unwrap()
        );
        assert_eq!(
            "rule_name_1__._",
            MatcherConfig::rule_name_from_filename("ascfb5.46_rule_name_1__._").unwrap()
        );
    }
}
