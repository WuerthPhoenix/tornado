use crate::config::filter::Filter;
use crate::config::rule::Rule;
use crate::error::MatcherError;
use log::{debug, info, trace};
use serde_derive::{Deserialize, Serialize};
use std::fs;

pub mod filter;
pub mod rule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatcherConfig {
    Filter(Filter),
    Rules(Vec<Rule>),
}

impl MatcherConfig {
    pub fn read_from_dir(dir: &str) -> Result<MatcherConfig, MatcherError> {
        if MatcherConfig::is_filter_dir(dir)? {
            return MatcherConfig::read_filter_from_dir(dir).and_then(|filter| Ok(MatcherConfig::Filter(filter)));
        }
        MatcherConfig::read_rules_from_dir(dir).and_then(|rules| Ok(MatcherConfig::Rules(rules)))
    }

    // Returns whether the directory contains a filter. Otherwise it contains rules.
    // This checks are performed to determine the folder content:
    // - It contains a filter if there is only one json file AND there are subdirectories. The result is true.
    // - It contains a rule set if there are no subdirectories. The result is false.
    // - It returns an error in every other case.
    fn is_filter_dir(dir: &str) -> Result<bool, MatcherError> {
        let paths = fs::read_dir(dir)
            .and_then(|entry_set| entry_set.collect::<Result<Vec<_>, _>>())
            .map_err(|e| MatcherError::ConfigurationError {
                message: format!("Error reading from config path [{}]: {}", dir, e),
            })?;

        let mut subdirectories_count = 0;
        let mut json_files_count = 0;

        for entry in paths {
            let path = entry.path();

            if path.is_dir() {
                subdirectories_count += 1;
            } else {
                let filename = path.to_str().ok_or_else(|| MatcherError::ConfigurationError {
                    message: format!("Error processing filename of file: [{}]", path.display()),
                })?;

                if filename.ends_with(".json") {
                    json_files_count += 1;
                }
            }
        }
        debug!(
            "Path {} contains {} file(s) and {} directories",
            dir, json_files_count, subdirectories_count
        );

        if subdirectories_count > 0 {
            if json_files_count == 1 {
                return Ok(true);
            }
            return Err(MatcherError::ConfigurationError {
                message: format!("Path {} contains {} file(s) and {} directories. Expected exactly one json filter file to be present.", dir, json_files_count, subdirectories_count),
            });
        }
        Ok(false)
    }

    fn read_rules_from_dir(dir: &str) -> Result<Vec<Rule>, MatcherError> {
        let mut paths = fs::read_dir(dir)
            .and_then(|entry_set| entry_set.collect::<Result<Vec<_>, _>>())
            .map_err(|e| MatcherError::ConfigurationError {
                message: format!("Error reading from config path [{}]: {}", dir, e),
            })?;

        // Sort by filename
        paths.sort_by_key(|dir| dir.path());

        let mut rules = vec![];

        for entry in paths {
            let path = entry.path();

            let filename = path.to_str().ok_or_else(|| MatcherError::ConfigurationError {
                message: format!("Error processing filename of file: [{}]", path.display()),
            })?;

            if !filename.ends_with(".json") {
                info!("Configuration file [{}] is ignored.", path.display());
                continue;
            }

            info!("Loading rule from file: [{}]", path.display());
            let rule_body =
                fs::read_to_string(&path).map_err(|e| MatcherError::ConfigurationError {
                    message: format!("Unable to open the file [{}]. Err: {}", path.display(), e),
                })?;

            trace!("Rule body: \n{}", rule_body);
            rules.push(Rule::from_json(&rule_body)?)
        }

        info!("Loaded {} rule(s) from [{}]", rules.len(), dir);

        Ok(rules)
    }

    fn read_filter_from_dir(dir: &str) -> Result<Filter, MatcherError> {
        let paths = fs::read_dir(dir)
            .and_then(|entry_set| entry_set.collect::<Result<Vec<_>, _>>())
            .map_err(|e| MatcherError::ConfigurationError {
                message: format!("Error reading from config path [{}]: {}", dir, e),
            })?;

        for entry in paths {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let filename = path.to_str().ok_or_else(|| MatcherError::ConfigurationError {
                message: format!("Error processing filename of file: [{}]", path.display()),
            })?;

            if !filename.ends_with(".json") {
                info!("Configuration file [{}] is ignored.", path.display());
                continue;
            }

            info!("Loading filter from file: [{}]", path.display());
            let filter_body =
                fs::read_to_string(&path).map_err(|e| MatcherError::ConfigurationError {
                    message: format!("Unable to open the file [{}]. Err: {}", path.display(), e),
                })?;

            trace!("Filter body: \n{}", filter_body);
            return Filter::from_json(&filter_body)
        };

        Err(MatcherError::ConfigurationError {
            message: format!("Config path [{}] contains no json files. Expected exactly one json filter file.", dir),
        })
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
            MatcherConfig::Rules(rules) => {
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
            MatcherConfig::Rules(rules) => {
                assert_eq!(0, rules.len());
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn should_read_filter_from_folder() {
        let path = "./test_resources/config_01";
        let config = MatcherConfig::read_from_dir(path).unwrap();

        match config {
            MatcherConfig::Filter(filter) => {
                assert_eq!("only_emails", filter.name);
            }
            _ => assert!(false),
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
        let path = "./test_resources/config_01";
        let config = MatcherConfig::read_from_dir(path).unwrap();

        match config {
            MatcherConfig::Filter(filter) => {
                assert_eq!("only_emails", filter.name);
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn is_filter_dir_should_return_true_if_one_file_and_one_subdir() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        fs::create_dir_all(&format!("{}/subdir", dir)).unwrap();
        fs::File::create(&format!("{}/file.json", dir)).unwrap();

        // Act
        let result = MatcherConfig::is_filter_dir(&dir);

        // Assert
        assert_eq!(Ok(true), result);
    }

    #[test]
    fn is_filter_dir_should_return_false_if_one_file_and_no_subdir() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        fs::File::create(&format!("{}/file.json", dir)).unwrap();

        // Act
        let result = MatcherConfig::is_filter_dir(&dir);

        // Assert
        assert_eq!(Ok(false), result);
    }

    #[test]
    fn is_filter_dir_should_return_false_if_many_files_and_no_subdir() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        fs::File::create(&format!("{}/file_01.json", dir)).unwrap();
        fs::File::create(&format!("{}/file_02.json", dir)).unwrap();
        fs::File::create(&format!("{}/file_03.json", dir)).unwrap();

        // Act
        let result = MatcherConfig::is_filter_dir(&dir);

        // Assert
        assert_eq!(Ok(false), result);
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
        let result = MatcherConfig::is_filter_dir(&dir);

        // Assert
        assert_eq!(Err(MatcherError::ConfigurationError {
            message: format!("Path {} contains {} file(s) and {} directories. Expected exactly one json filter file to be present.", dir, 2, 2),
        }), result);
    }

    #[test]
    fn is_filter_dir_should_return_error_if_no_files_but_subdirs() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();

        fs::create_dir_all(&format!("{}/subdir1", dir)).unwrap();
        fs::create_dir_all(&format!("{}/subdir2", dir)).unwrap();

        // Act
        let result = MatcherConfig::is_filter_dir(&dir);

        // Assert
        assert_eq!(Err(MatcherError::ConfigurationError {
            message: format!("Path {} contains {} file(s) and {} directories. Expected exactly one json filter file to be present.", dir, 0, 2),
        }), result);
    }
}
