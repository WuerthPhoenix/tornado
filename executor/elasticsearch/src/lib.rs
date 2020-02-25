use log::*;
use tornado_common_api::{Action};
use tornado_executor_common::{Executor, ExecutorError};
use reqwest::Client;

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

        let data = action.payload.get("data").ok_or_else(|| {
            ExecutorError::MissingArgumentError { message: "data field is missing".to_string() }
        })?;

        let index_name = "tornado";
        let endpoint = format!("http://127.0.0.1:9200/{}/_doc/", index_name);

        let client = Client::new();
        let res = client.post(&endpoint)
            .json(&data)
            .send().map_err(|err| ExecutorError::ActionExecutionError { message: format!("Error while sending document to Elasticsearch. Err: {}", err) })?;

        if !res.status().is_success(){
            Err(ExecutorError::ActionExecutionError { message: format!("Error while sending document to Elasticsearch. Response: {:?}", res) })
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

    #[test]
    fn should_send_document_to_elasticsearch() {
        // Arrange
        let mut executor = ElasticsearchExecutor {};
        let mut action = Action { id: "elasticsearch".to_string(), payload: HashMap::new() };
        let mut es_document = HashMap::new();
        es_document.insert("message".to_owned(), Value::Text("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::Text("myuser".to_owned()));

        action.payload.insert("data".to_owned(), Value::Map(es_document));

        // Act
        let result = executor.execute(action);

        assert_eq!(result, Ok(()));
    }
}
