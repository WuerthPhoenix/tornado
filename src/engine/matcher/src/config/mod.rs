use crate::config::rule::Rule;
use crate::error::MatcherError;
use log::{info, trace};
use serde_derive::{Deserialize, Serialize};
use std::fs;

pub mod filter;
pub mod rule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatcherConfig {
    Rules(Vec<Rule>),
}

impl MatcherConfig {
    pub fn read_from_dir(dir: &str) -> Result<MatcherConfig, MatcherError> {
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

        Ok(MatcherConfig::Rules(rules))
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_read_from_folder_sorting_by_filename() {
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
}
