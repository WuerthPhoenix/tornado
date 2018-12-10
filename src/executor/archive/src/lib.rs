#[macro_use]
extern crate log;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tornado_common_api;
extern crate tornado_executor_common;

use std::collections::HashMap;
use std::fs::create_dir_all;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError};

pub mod config;
mod paths;

pub const ARCHIVE_TYPE_KEY: &str = "archive_type";
pub const EVENT_KEY: &str = "event";

pub struct ArchiveExecutor {
    pub base_path: String,
    pub default_path: String,
    paths: HashMap<String, paths::PathMatcher>,
}

impl ArchiveExecutor {
    pub fn new(config: &config::ArchiveConfig) -> ArchiveExecutor {
        let builder = paths::PathMatcherBuilder::new();
        let paths = config
            .paths
            .iter()
            .map(|(key, value)| (key.to_owned(), builder.build(value.to_owned())))
            .collect::<HashMap<String, paths::PathMatcher>>();
        ArchiveExecutor {
            base_path: config.base_path.clone(),
            default_path: config.default_path.clone(),
            paths,
        }
    }

    fn write(&mut self, relative_path: &str, buf: &[u8]) -> Result<(), ExecutorError> {
        let path = format!("{}{}{}", self.base_path, std::path::MAIN_SEPARATOR, relative_path);

        if path.contains(r"\..") || path.contains("/..") {
            return Err(ExecutorError::ActionExecutionError {
                message: format!("Suspicious path [{:?}]. It could be an attempt to write outside the main directory.", &path),
            });
        }

        let path = Path::new(&path);

        if let Some(parent) = path.parent() {
            create_dir_all(&parent).map_err(|err| ExecutorError::ActionExecutionError {
                message: format!(
                    "Cannot create required directories for path [{:?}]: {}",
                    &path, err
                ),
            })?;
        }

        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| file.write_all(buf))
            .map_err(|err| ExecutorError::ActionExecutionError {
                message: format!("Cannot write to file: {}", err),
            })
    }
}

impl Executor for ArchiveExecutor {
    fn execute(&mut self, action: &Action) -> Result<(), ExecutorError> {
        debug!("ArchiveExecutor - received action: \n{:#?}", action);

        let archive_type =
            action.payload.get(ARCHIVE_TYPE_KEY).and_then(|value| value.text()).ok_or_else(
                || ExecutorError::ActionExecutionError {
                    message: format!(
                        "[{}] key not found be in action payload or it is not a String.",
                        ARCHIVE_TYPE_KEY
                    ),
                },
            )?;

        let mut event_bytes = action
            .payload
            .get(EVENT_KEY)
            .ok_or_else(|| ExecutorError::ActionExecutionError {
                message: format!("Expected the [{}] key to be in action payload.", EVENT_KEY),
            })
            .and_then(|value| {
                serde_json::to_vec(value).map_err(|err| ExecutorError::ActionExecutionError {
                    message: format!("Cannot deserialize event:{}", err),
                })
            })?;

        event_bytes.push(b'\n');

        let path = match self.paths.get(archive_type) {
            Some(path_matcher) => path_matcher.build_path(&action.payload).unwrap_or_else(|err| {
                warn!("Fallback to default path: {}", err);
                // ToDo: clone to be removed when edition 2018 is enabled
                self.default_path.clone()
            }),
            // ToDo: clone to be removed when edition 2018 is enabled
            None => self.default_path.clone(),
        };

        self.write(&path, &event_bytes)?;

        Ok(())
    }
}

#[cfg(test)]
extern crate tempfile;

#[cfg(test)]
mod test {

    use super::*;
    use std::fs;
    use std::io::{BufRead, BufReader};
    use tornado_common_api::Event;
    use tornado_common_api::Value;

    #[test]
    fn should_write_to_expected_path() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let mut config = config::ArchiveConfig {
            base_path: dir.to_owned(),
            default_path: "/default/file.out".to_owned(),
            paths: HashMap::new(),
        };

        config.paths.insert("one".to_owned(), "/one/${key_one}/${key_two}.log".to_owned());

        let expected_path = format!("{}/{}", &dir, "one/first/second.log");

        println!("Expected file path: [{}]", &expected_path);

        let mut archiver = ArchiveExecutor::new(&config);

        let event = Event::new("event-name");
        let mut action = Action::new("action");
        action.payload.insert(EVENT_KEY.to_owned(), event.clone().into());
        action.payload.insert(ARCHIVE_TYPE_KEY.to_owned(), Value::Text("one".to_owned()));
        action.payload.insert("key_one".to_owned(), Value::Text("first".to_owned()));
        action.payload.insert("key_two".to_owned(), Value::Text("second".to_owned()));

        // Act
        let result = archiver.execute(&action);

        // Assert
        assert!(result.is_ok());

        let file_content = std::fs::read_to_string(&expected_path).unwrap();
        let event_from_file = serde_json::from_str::<Event>(&file_content).unwrap();

        assert_eq!(event, event_from_file);
    }

    #[test]
    fn should_write_an_event_per_line() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let mut config = config::ArchiveConfig {
            base_path: dir.to_owned(),
            default_path: "/default/file.out".to_owned(),
            paths: HashMap::new(),
        };

        config.paths.insert("one".to_owned(), "/one/${key_one}/${key_two}.log".to_owned());

        let expected_path = format!("{}/{}", &dir, "one/first/second.log");

        println!("Expected file path: [{}]", &expected_path);

        let mut archiver = ArchiveExecutor::new(&config);

        let attempts = 10;
        let mut sent_events = vec![];
        let mut read_lines = vec![];

        // Act
        for i in 0..attempts {
            let event = Event::new(format!("event-name-{}", i));
            sent_events.push(event.clone());
            let mut action = Action::new(format!("action-{}", i));
            action.payload.insert(EVENT_KEY.to_owned(), event.clone().into());
            action.payload.insert(ARCHIVE_TYPE_KEY.to_owned(), Value::Text("one".to_owned()));
            action.payload.insert("key_one".to_owned(), Value::Text("first".to_owned()));
            action.payload.insert("key_two".to_owned(), Value::Text("second".to_owned()));
            archiver.execute(&action).unwrap()
        }

        let file = fs::File::open(&expected_path).unwrap();
        for line in BufReader::new(file).lines() {
            let line_string = line.unwrap();
            println!("Read line: {}", &line_string);
            read_lines.push(line_string);
        }

        // Assert
        assert_eq!(attempts, sent_events.len());
        assert_eq!(attempts, read_lines.len());

        for i in 0..attempts {
            let event_from_file =
                serde_json::from_str::<Event>(read_lines.get(i).unwrap()).unwrap();
            assert_eq!(sent_events.get(i).unwrap(), &event_from_file)
        }
    }

    #[test]
    fn should_not_allow_writing_outside_the_base_path() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let mut config = config::ArchiveConfig {
            base_path: dir.to_owned(),
            default_path: "/default/file.out".to_owned(),
            paths: HashMap::new(),
        };

        config.paths.insert("one".to_owned(), "/one/${key_one}/${key_two}.log".to_owned());

        let mut archiver = ArchiveExecutor::new(&config);

        let event = Event::new("event-name");
        let mut action = Action::new("action");
        action.payload.insert(EVENT_KEY.to_owned(), event.clone().into());
        action.payload.insert(ARCHIVE_TYPE_KEY.to_owned(), Value::Text("one".to_owned()));
        action.payload.insert("key_one".to_owned(), Value::Text("../".to_owned()));
        action.payload.insert("key_two".to_owned(), Value::Text("second".to_owned()));

        // Act
        let result = archiver.execute(&action);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_fallback_to_default_path_if_cannot_resolve_params() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let mut config = config::ArchiveConfig {
            base_path: dir.to_owned(),
            default_path: "/default/file.out".to_owned(),
            paths: HashMap::new(),
        };

        config.paths.insert("one".to_owned(), "/one/${key_one}/${key_two}.log".to_owned());

        let expected_path = format!("{}/{}", &dir, "/default/file.out");

        println!("Expected file path: [{}]", &expected_path);

        let mut archiver = ArchiveExecutor::new(&config);

        let event = Event::new("event-name");
        let mut action = Action::new("action");
        action.payload.insert(EVENT_KEY.to_owned(), event.clone().into());
        action.payload.insert(ARCHIVE_TYPE_KEY.to_owned(), Value::Text("one".to_owned()));

        // Act
        let result = archiver.execute(&action);

        // Assert
        assert!(result.is_ok());

        let file_content = std::fs::read_to_string(&expected_path).unwrap();
        let event_from_file = serde_json::from_str::<Event>(&file_content).unwrap();

        assert_eq!(event, event_from_file);
    }
}
