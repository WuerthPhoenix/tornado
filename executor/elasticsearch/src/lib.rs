use log::*;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::Client;
use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError};

const ENDPOINT_KEY: &str = "endpoint";
const DATA_KEY: &str = "data";
const INDEX_KEY: &str = "index";

/// An executor that sends data to elasticsearch
#[derive(Default)]
pub struct ElasticsearchExecutor {}

impl ElasticsearchExecutor {
    pub fn new() -> ElasticsearchExecutor {
        Default::default()
    }
}

impl std::fmt::Display for ElasticsearchExecutor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("ElasticsearchExecutor")?;
        Ok(())
    }
}

impl Executor for ElasticsearchExecutor {
    fn execute(&mut self, action: Action) -> Result<(), ExecutorError> {
        trace!("ElasticsearchExecutor - received action: \n[{:?}]", action);

        let data = action.payload.get(DATA_KEY).ok_or_else(|| {
            ExecutorError::MissingArgumentError { message: "data field is missing".to_string() }
        })?;

        let endpoint =
            action.payload.get(ENDPOINT_KEY).and_then(|val| val.get_text()).ok_or_else(|| {
                ExecutorError::MissingArgumentError {
                    message: "endpoint field is missing".to_string(),
                }
            })?;

        let index_name =
            action.payload.get(INDEX_KEY).and_then(|val| val.get_text()).ok_or_else(|| {
                ExecutorError::MissingArgumentError {
                    message: "index field is missing".to_string(),
                }
            })?;

        let endpoint =
            format!("{}/{}/_doc/", endpoint, utf8_percent_encode(index_name, NON_ALPHANUMERIC));

        let client = Client::new();
        let res = client.post(&endpoint).json(&data).send().map_err(|err| {
            ExecutorError::ActionExecutionError {
                message: format!("Error while sending document to Elasticsearch. Err: {}", err),
            }
        })?;

        if !res.status().is_success() {
            Err(ExecutorError::ActionExecutionError {
                message: format!(
                    "Error while sending document to Elasticsearch. Response: {:?}",
                    res
                ),
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;
    use tornado_common_api::Value;

    //    This can be used for local testing. It requires Elasticsearch running on localhost
    //    #[test]
    //    fn should_send_document_to_elasticsearch() {
    //        // Arrange
    //        let mut executor = ElasticsearchExecutor {};
    //        let mut action = Action { id: "elasticsearch".to_string(), payload: HashMap::new() };
    //        let mut es_document = HashMap::new();
    //        es_document.insert("message".to_owned(), Value::Text("message to elasticsearch".to_owned()));
    //        es_document.insert("user".to_owned(), Value::Text("myuser".to_owned()));
    //
    //        action.payload.insert("data".to_owned(), Value::Map(es_document));
    //        action.payload.insert("index".to_owned(), Value::Text("tornàdo".to_owned()));
    //        action.payload.insert("endpoint".to_owned(), Value::Text("http://127.0.0.1:9200".to_owned()));
    //
    //        // Act
    //        let result = executor.execute(action);
    //
    //        // Assert
    //        assert!(result.is_ok());
    //    }

    #[test]
    fn should_fail_if_index_is_missing() {
        // Arrange
        let mut executor = ElasticsearchExecutor {};
        let mut action = Action { id: "elasticsearch".to_string(), payload: HashMap::new() };
        let mut es_document = HashMap::new();
        es_document
            .insert("message".to_owned(), Value::Text("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::Text("myuser".to_owned()));

        action.payload.insert("data".to_owned(), Value::Map(es_document));
        action
            .payload
            .insert("endpoint".to_owned(), Value::Text("http://127.0.0.1:9200".to_owned()));

        // Act
        let result = executor.execute(action);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_if_endpoint_is_missing() {
        // Arrange
        let mut executor = ElasticsearchExecutor {};
        let mut action = Action { id: "elasticsearch".to_string(), payload: HashMap::new() };
        let mut es_document = HashMap::new();
        es_document
            .insert("message".to_owned(), Value::Text("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::Text("myuser".to_owned()));

        action.payload.insert("data".to_owned(), Value::Map(es_document));
        action.payload.insert("index".to_owned(), Value::Text("tornàdo".to_owned()));

        // Act
        let result = executor.execute(action);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_if_data_is_missing() {
        // Arrange
        let mut executor = ElasticsearchExecutor {};
        let mut action = Action { id: "elasticsearch".to_string(), payload: HashMap::new() };
        let mut es_document = HashMap::new();
        es_document
            .insert("message".to_owned(), Value::Text("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::Text("myuser".to_owned()));

        action
            .payload
            .insert("endpoint".to_owned(), Value::Text("http://127.0.0.1:9200".to_owned()));
        action.payload.insert("index".to_owned(), Value::Text("tornàdo".to_owned()));

        // Act
        let result = executor.execute(action);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_if_index_is_not_text() {
        // Arrange
        let mut executor = ElasticsearchExecutor {};
        let mut action = Action { id: "elasticsearch".to_string(), payload: HashMap::new() };
        let mut es_document = HashMap::new();
        es_document
            .insert("message".to_owned(), Value::Text("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::Text("myuser".to_owned()));

        action.payload.insert("data".to_owned(), Value::Map(es_document));
        action.payload.insert("index".to_owned(), Value::Array(vec![]));
        action
            .payload
            .insert("endpoint".to_owned(), Value::Text("http://127.0.0.1:9200".to_owned()));

        // Act
        let result = executor.execute(action);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_fail_if_endpoint_is_not_text() {
        // Arrange
        let mut executor = ElasticsearchExecutor {};
        let mut action = Action { id: "elasticsearch".to_string(), payload: HashMap::new() };
        let mut es_document = HashMap::new();
        es_document
            .insert("message".to_owned(), Value::Text("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::Text("myuser".to_owned()));

        action.payload.insert("data".to_owned(), Value::Map(es_document));
        action.payload.insert("index".to_owned(), Value::Text("tornàdo".to_owned()));
        action.payload.insert("endpoint".to_owned(), Value::Bool(false));

        // Act
        let result = executor.execute(action);

        // Assert
        assert!(result.is_err());
    }
}
