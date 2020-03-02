use log::*;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use tornado_common_api::Action;
use tornado_executor_common::{Executor, ExecutorError};
use std::fs::File;
use std::io::Read;
use reqwest::{Client, Identity, Certificate};
use std::str::from_utf8;

const ENDPOINT_KEY: &str = "endpoint";
const DATA_KEY: &str = "data";
const INDEX_KEY: &str = "index";
const CERTIFICATE_KEY: &str = "cert_path";
const PRIVATE_CERTIFICATE_KEY_KEY: &str = "private_key_path";

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

        let certificate_path =
            action.payload.get(CERTIFICATE_KEY).and_then(|val| val.get_text()).ok_or_else(|| {
                ExecutorError::MissingArgumentError {
                    message: "cert_path field is missing".to_string(),
                }
            })?;

        let private_certificate_key_path =
            action.payload.get(PRIVATE_CERTIFICATE_KEY_KEY).and_then(|val| val.get_text()).ok_or_else(|| {
                ExecutorError::MissingArgumentError {
                    message: "private_key_path field is missing".to_string(),
                }
            })?;

        let endpoint =
            format!("{}/{}/_doc/", endpoint, utf8_percent_encode(index_name, NON_ALPHANUMERIC));

        let mut buf = Vec::new();
        File::open(private_certificate_key_path).and_then(|mut file| file.read_to_end(&mut buf)).map_err(|err| {
            ExecutorError::ActionExecutionError {
                message: format!("Error while reading private key file {}. Err: {}", private_certificate_key_path, err),
            }
        })?;

        File::open(certificate_path).and_then(|mut file| file.read_to_end(&mut buf)).map_err(|err| {
            ExecutorError::ActionExecutionError {
                message: format!("Error while reading certificate file {}. Err: {}", certificate_path, err),
            }
        })?;

        let mut pk  = Vec::new();
        File::open("/home/damianochini/projects/neteye-4/tornado/executor/elasticsearch/identity.pfx").and_then(|mut file| file.read_to_end(&mut pk)).map_err(|err| {
            ExecutorError::ActionExecutionError {
                message: format!("Error while reading private key file {}. Err: {}", private_certificate_key_path, err),
            }
        })?;

        let identity = Identity::from_pkcs12_der(&pk, "admin").map_err(|err| {
            ExecutorError::ActionExecutionError {
                message: format!("Error while creating identity. Err: {}", err),
            }
        })?;

        let mut cert_buffer = Vec::new();
        File::open(certificate_path).and_then(|mut file| file.read_to_end(&mut cert_buffer)).map_err(|err| {
            ExecutorError::ActionExecutionError {
                message: format!("Error while reading certificate file {}. Err: {}", certificate_path, err),
            }
        })?;

        let certificate = Certificate::from_pem(&cert_buffer).map_err(|err| {
            ExecutorError::ActionExecutionError {
                message: format!("Error while creating certificate. Err: {}", err),
            }
        })?;

        let client = reqwest::Client::builder().identity(identity)//.add_root_certificate(certificate)
            .use_default_tls()
            .build().map_err(|err| {
            ExecutorError::ActionExecutionError {
                message: format!("Error while building reqwest client. Err: {}", err),
            }
        })?;

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

//        This can be used for local testing. It requires Elasticsearch running on localhost
        #[test]
        fn should_send_document_to_elasticsearch() {
            // Arrange
            let mut executor = ElasticsearchExecutor {};
            let mut action = Action { id: "elasticsearch".to_string(), payload: HashMap::new() };
            let mut es_document = HashMap::new();
            es_document.insert("message".to_owned(), Value::Text("message to elasticsearch".to_owned()));
            es_document.insert("user".to_owned(), Value::Text("myuser".to_owned()));

            action.payload.insert("data".to_owned(), Value::Map(es_document));
            action.payload.insert("index".to_owned(), Value::Text("tornado-example".to_owned()));
            action.payload.insert("endpoint".to_owned(), Value::Text("https://elasticsearch.neteyelocal:9200".to_owned()));
            action.payload.insert("cert_path".to_owned(), Value::Text("/neteye/shared/tornado/conf/certs/tornado.crt.pem".to_owned()));
            action.payload.insert("private_key_path".to_owned(), Value::Text("/neteye/shared/tornado/conf/certs/private/tornado.key.pem".to_owned()));

            // Act
            let result = executor.execute(action);
            result.unwrap();
            // Assert
//            assert!(result.is_ok());
        }

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
