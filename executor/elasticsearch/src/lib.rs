use log::*;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{Executor, ExecutorError};

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

        Ok(())
    }
}

impl ElasticsearchExecutor {
    fn execute_async(&self, action: Action) -> Result<(), ExecutorError> {
        let data = action.payload.get("data").ok_or_else(|| {
            ExecutorError::MissingArgumentError { message: "data field is missing".to_string() }
        })?;
        let endpoint = "http://127.0.0.1:9200";

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn should_send_document_to_elasticsearch() {
        // Arrange
        let executor = ElasticsearchExecutor {};
        let mut action = Action { id: "elasticsearch".to_string(), payload: HashMap::new() };
        action.payload.insert("data".to_owned(), Value::Text("ciao elasticsearch".to_owned()));

        // Act
        let result = executor.execute_async(action);
    }
}
