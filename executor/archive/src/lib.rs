use log::*;
use lru_time_cache::Entry;
use lru_time_cache::LruCache;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::create_dir_all;
use tokio::fs::File;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;
use tornado_common_api::Action;
use tornado_executor_common::{ExecutorError, StatefulExecutor};
use tracing::instrument;

pub mod config;
mod paths;

pub const ARCHIVE_TYPE_KEY: &str = "archive_type";
pub const EVENT_KEY: &str = "event";

pub struct ArchiveExecutor {
    pub base_path: String,
    pub default_path: String,
    paths: HashMap<String, paths::PathMatcher>,
    file_cache: LruCache<String, BufWriter<File>>,
}

impl std::fmt::Display for ArchiveExecutor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("ArchiveExecutor(base_path='")?;
        fmt.write_str(&self.base_path)?;
        fmt.write_str("')")?;
        Ok(())
    }
}

impl ArchiveExecutor {
    pub fn new(config: &config::ArchiveConfig) -> ArchiveExecutor {
        let builder = paths::PathMatcherBuilder::new();
        let paths = config
            .paths
            .iter()
            .map(|(key, value)| (key.to_owned(), builder.build(value.to_owned())))
            .collect::<HashMap<String, paths::PathMatcher>>();

        let time_to_live = ::std::time::Duration::from_secs(config.file_cache_ttl_secs);
        let file_cache =
            LruCache::with_expiry_duration_and_capacity(time_to_live, config.file_cache_size);

        ArchiveExecutor {
            base_path: config.base_path.clone(),
            default_path: config.default_path.clone(),
            paths,
            file_cache,
        }
    }

    #[instrument(level = "debug", name = "Extract parameters for Executor", skip_all)]
    fn extract_params_from_payload(
        &self,
        action: &Action,
    ) -> Result<(Option<String>, Vec<u8>), ExecutorError> {
        let path = match action
            .payload
            .get(ARCHIVE_TYPE_KEY)
            .and_then(tornado_common_api::ValueExt::get_text)
        {
            Some(archive_type) => match self.paths.get(archive_type) {
                Some(path_matcher) => path_matcher.build_path(&action.payload).map(Some),
                None => Err(ExecutorError::ActionExecutionError {
                    can_retry: false,
                    message: format!(
                        "Cannot find mapping for {} value: [{}]",
                        ARCHIVE_TYPE_KEY, archive_type
                    ),
                    code: None,
                    data: Default::default(),
                }),
            },
            None => Ok(None),
        }?;

        let mut event_bytes = action
            .payload
            .get(EVENT_KEY)
            .ok_or_else(|| ExecutorError::ActionExecutionError {
                can_retry: false,
                message: format!("Expected the [{}] key to be in action payload.", EVENT_KEY),
                code: None,
                data: Default::default(),
            })
            .and_then(|value| {
                serde_json::to_vec(value).map_err(|err| ExecutorError::ActionExecutionError {
                    can_retry: false,
                    message: format!("Cannot deserialize event:{}", err),
                    code: None,
                    data: Default::default(),
                })
            })?;

        event_bytes.push(b'\n');

        Ok((path, event_bytes))
    }

    #[instrument(level = "debug", name = "ArchiveExecutor", skip_all, fields(otel.name = format!("Archive Event to: {}", absolute_path_string).as_str()))]
    async fn write(
        &mut self,
        absolute_path_string: String,
        buf: &[u8],
    ) -> Result<(), ExecutorError> {
        let buf_writer = match self.file_cache.entry(absolute_path_string.clone()) {
            Entry::Occupied(occupied) => occupied.into_mut(),
            Entry::Vacant(vacant) => {
                if absolute_path_string.contains(r"\..") || absolute_path_string.contains("/..") {
                    return Err(ExecutorError::ActionExecutionError {
                        can_retry: false,
                        message: format!("Suspicious path [{:?}]. It could be an attempt to write outside the main directory.", &absolute_path_string),
                        code: None,
                        data: Default::default(),
                    });
                }

                let path = Path::new(&absolute_path_string);

                if let Some(parent) = path.parent() {
                    create_dir_all(&parent).await.map_err(|err| {
                        ExecutorError::ActionExecutionError {
                            can_retry: true,
                            message: format!(
                                "Cannot create required directories for path [{:?}]: {}",
                                &path, err
                            ),
                            code: None,
                            data: Default::default(),
                        }
                    })?;
                }

                let file = OpenOptions::new().create(true).append(true).open(&path).await.map_err(
                    |err| ExecutorError::ActionExecutionError {
                        can_retry: true,
                        message: format!("Cannot open file [{}]: {}", &absolute_path_string, err),
                        code: None,
                        data: Default::default(),
                    },
                )?;

                vacant.insert(BufWriter::new(file))
            }
        };

        buf_writer.write_all(buf).await.map_err(|err| ExecutorError::ActionExecutionError {
            can_retry: true,
            message: format!("Cannot write to file [{}]: {}", &absolute_path_string, err),
            code: None,
            data: Default::default(),
        })?;
        buf_writer.flush().await.map_err(|err| ExecutorError::ActionExecutionError {
            can_retry: true,
            message: format!("Cannot flush file [{}]: {}", &absolute_path_string, err),
            code: None,
            data: Default::default(),
        })
    }
}

#[async_trait::async_trait(?Send)]
impl StatefulExecutor for ArchiveExecutor {
    #[tracing::instrument(level = "info", skip_all, err, fields(otel.name = format!("Execute Action: {}", &action.id).as_str(), otel.kind = "Consumer"))]
    async fn execute(&mut self, action: Arc<Action>) -> Result<(), ExecutorError> {
        trace!("ArchiveExecutor - received action: \n{:?}", action);

        let (path, event_bytes) = self.extract_params_from_payload(&action)?;
        let absolute_path_string = format!(
            "{}{}{}",
            self.base_path,
            std::path::MAIN_SEPARATOR,
            path.map(std::borrow::Cow::Owned)
                .unwrap_or_else(|| std::borrow::Cow::Borrowed(&self.default_path))
        );

        self.write(absolute_path_string, &event_bytes).await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use serde_json::json;
    use tokio::fs::{self, read_to_string};
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tornado_common_api::Event;
    use tornado_common_api::Value;

    #[tokio::test]
    async fn should_write_to_expected_path() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let mut config = config::ArchiveConfig {
            base_path: dir.to_owned(),
            default_path: "/default/file.out".to_owned(),
            paths: HashMap::new(),
            file_cache_size: 10,
            file_cache_ttl_secs: 1,
        };

        config.paths.insert("one".to_owned(), "/one/${key_one}/${key_two}.log".to_owned());

        let expected_path = format!("{}/{}", &dir, "one/first/second.log");

        println!("Expected file path: [{}]", &expected_path);

        let mut archiver = ArchiveExecutor::new(&config);

        let event = Event::new("event-name");
        let mut action = Action::new("action");
        action.payload.insert(EVENT_KEY.to_owned(), json!(event.clone()));
        action.payload.insert(ARCHIVE_TYPE_KEY.to_owned(), Value::String("one".to_owned()));
        action.payload.insert("key_one".to_owned(), Value::String("first".to_owned()));
        action.payload.insert("key_two".to_owned(), Value::String("second".to_owned()));

        // Act
        let result = archiver.execute(action.into()).await;

        // Assert
        assert!(result.is_ok());

        let file_content = read_to_string(&expected_path).await.unwrap();
        let event_from_file = serde_json::from_str::<Event>(&file_content).unwrap();

        assert_eq!(event, event_from_file);
    }

    #[tokio::test]
    async fn should_write_an_event_per_line() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let mut config = config::ArchiveConfig {
            base_path: dir.to_owned(),
            default_path: "/default/file.out".to_owned(),
            paths: HashMap::new(),
            file_cache_size: 10,
            file_cache_ttl_secs: 1,
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
            action.payload.insert(EVENT_KEY.to_owned(), json!(event.clone()));
            action.payload.insert(ARCHIVE_TYPE_KEY.to_owned(), Value::String("one".to_owned()));
            action.payload.insert("key_one".to_owned(), Value::String("first".to_owned()));
            action.payload.insert("key_two".to_owned(), Value::String("second".to_owned()));
            archiver.execute(action.into()).await.unwrap()
        }

        let file = fs::File::open(&expected_path).await.unwrap();
        let mut lines = BufReader::new(file).lines();
        while let Some(line_string) = lines.next_line().await.unwrap() {
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

    #[tokio::test]
    async fn should_not_allow_writing_outside_the_base_path() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let mut config = config::ArchiveConfig {
            base_path: dir.to_owned(),
            default_path: "/default/file.out".to_owned(),
            paths: HashMap::new(),
            file_cache_size: 10,
            file_cache_ttl_secs: 1,
        };

        config.paths.insert("one".to_owned(), "/one/${key_one}/${key_two}.log".to_owned());

        let mut archiver = ArchiveExecutor::new(&config);

        let event = Event::new("event-name");
        let mut action = Action::new("action");
        action.payload.insert(EVENT_KEY.to_owned(), json!(event.clone()));
        action.payload.insert(ARCHIVE_TYPE_KEY.to_owned(), Value::String("one".to_owned()));
        action.payload.insert("key_one".to_owned(), Value::String("../".to_owned()));
        action.payload.insert("key_two".to_owned(), Value::String("second".to_owned()));

        // Act
        let result = archiver.execute(action.into()).await;

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_return_error_if_cannot_resolve_params() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let mut config = config::ArchiveConfig {
            base_path: dir.to_owned(),
            default_path: "/default/file.out".to_owned(),
            paths: HashMap::new(),
            file_cache_size: 10,
            file_cache_ttl_secs: 1,
        };

        config.paths.insert("one".to_owned(), "/one/${key_one}/${key_two}.log".to_owned());

        let mut archiver = ArchiveExecutor::new(&config);

        let event = Event::new("event-name");
        let mut action = Action::new("action");
        action.payload.insert(EVENT_KEY.to_owned(), json!(event.clone()));
        action.payload.insert(ARCHIVE_TYPE_KEY.to_owned(), Value::String("one".to_owned()));

        // Act
        let result = archiver.execute(action.into()).await;

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_return_error_if_action_type_is_not_mapped() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let mut config = config::ArchiveConfig {
            base_path: dir.to_owned(),
            default_path: "/default/file.out".to_owned(),
            paths: HashMap::new(),
            file_cache_size: 10,
            file_cache_ttl_secs: 1,
        };

        config.paths.insert("one".to_owned(), "/one/${key_one}/${key_two}.log".to_owned());

        let mut archiver = ArchiveExecutor::new(&config);

        let event = Event::new("event-name");
        let mut action = Action::new("action");
        action.payload.insert(EVENT_KEY.to_owned(), json!(event.clone()));
        action.payload.insert(ARCHIVE_TYPE_KEY.to_owned(), Value::String("two".to_owned()));

        // Act
        let result = archiver.execute(action.into()).await;

        // Assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_use_default_if_archive_type_not_specified() {
        // Arrange
        let tempdir = tempfile::tempdir().unwrap();
        let dir = tempdir.path().to_str().unwrap().to_owned();
        let config = config::ArchiveConfig {
            base_path: dir.to_owned(),
            default_path: "/default/file.out".to_owned(),
            paths: HashMap::new(),
            file_cache_size: 10,
            file_cache_ttl_secs: 1,
        };

        let expected_path = format!("{}/{}", &dir, "/default/file.out");
        println!("Expected file path: [{}]", &expected_path);

        let mut archiver = ArchiveExecutor::new(&config);

        let event = Event::new("event-name");
        let mut action = Action::new("action");
        action.payload.insert(EVENT_KEY.to_owned(), json!(event.clone()));

        // Act
        let result = archiver.execute(action.into()).await;

        // Assert
        assert!(result.is_ok());

        let file_content = read_to_string(&expected_path).await.unwrap();
        let event_from_file = serde_json::from_str::<Event>(&file_content).unwrap();

        assert_eq!(event, event_from_file);
    }
}
