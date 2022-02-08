use log::*;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::{Certificate, Client, Identity};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tornado_common_api::{Action, ValueExt};
use tornado_executor_common::{ExecutorError, StatelessExecutor};

pub mod config;

const ENDPOINT_KEY: &str = "endpoint";
const DATA_KEY: &str = "data";
const INDEX_KEY: &str = "index";
const AUTH_KEY: &str = "auth";

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ElasticsearchAuthentication {
    PemCertificatePath {
        certificate_path: String,
        private_key_path: String,
        ca_certificate_path: String,
    },
    None,
}

impl ElasticsearchAuthentication {
    pub async fn new_client(&self) -> Result<Client, ExecutorError> {
        match self {
            ElasticsearchAuthentication::PemCertificatePath {
                certificate_path,
                private_key_path,
                ca_certificate_path,
            } => {
                debug!(
                    "ElasticsearchAuthentication - Creating new PemCertificatePath client from paths: {}, {}, {}",
                    certificate_path, private_key_path, ca_certificate_path,
                );
                PemCertificateData::from_fs(certificate_path, private_key_path, ca_certificate_path)
                    .await?
                    .new_client()
            }
            ElasticsearchAuthentication::None => {
                debug!("ElasticsearchAuthentication - Creating new client with no authentication",);
                Ok(Client::new())
            }
        }
    }
}

struct PemCertificateData {
    certificate_with_private_key: Vec<u8>,
    ca_certificate: Vec<u8>,
}

impl PemCertificateData {
    pub async fn from_fs(
        certificate_path: &str,
        private_key_path: &str,
        ca_certificate_path: &str,
    ) -> Result<Self, ExecutorError> {
        let mut certificate_with_private_key = vec![];
        read_file(certificate_path, &mut certificate_with_private_key).await?;
        read_file(private_key_path, &mut certificate_with_private_key).await?;

        let mut ca_certificate = vec![];
        read_file(ca_certificate_path, &mut ca_certificate).await?;

        Ok(PemCertificateData { certificate_with_private_key, ca_certificate })
    }

    pub fn new_client(&self) -> Result<Client, ExecutorError> {
        let identity = Identity::from_pem(&self.certificate_with_private_key).map_err(|err| {
            ExecutorError::ConfigurationError {
                message: format!("Error while creating client identity. Err: {:?}", err),
            }
        })?;
        let ca_certificate = Certificate::from_pem(&self.ca_certificate).map_err(|err| {
            ExecutorError::ConfigurationError {
                message: format!("Error while creating ca certificate. Err: {:?}", err),
            }
        })?;

        Client::builder()
            .identity(identity)
            .add_root_certificate(ca_certificate)
            .use_rustls_tls()
            .build()
            .map_err(|err| ExecutorError::ConfigurationError {
                message: format!("Error while building reqwest client. Err: {:?}", err),
            })
    }
}
/// An executor that sends data to elasticsearch
#[derive(Clone)]
pub struct ElasticsearchExecutor {
    default_client: Option<Client>,
}

impl ElasticsearchExecutor {
    pub async fn new(
        es_authentication: Option<ElasticsearchAuthentication>,
    ) -> Result<ElasticsearchExecutor, ExecutorError> {
        debug!("ElasticsearchExecutor - Creating new Elasticsearch executor");
        let default_client = match es_authentication {
            Some(es_authentication) => Some(es_authentication.new_client().await?),
            None => None,
        };

        Ok(ElasticsearchExecutor { default_client })
    }
}

async fn read_file(path: &str, buf: &mut Vec<u8>) -> Result<usize, ExecutorError> {
    let mut file =
        tokio::fs::File::open(path).await.map_err(|err| ExecutorError::ConfigurationError {
            message: format!("Error while reading file {}. Err: {:?}", path, err),
        })?;
    file.read_to_end(buf).await.map_err(|err| ExecutorError::ConfigurationError {
        message: format!("Error while reading file {}. Err: {:?}", path, err),
    })
}

impl std::fmt::Display for ElasticsearchExecutor {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("ElasticsearchExecutor")?;
        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl StatelessExecutor for ElasticsearchExecutor {
    async fn execute(&self, action: Arc<Action>) -> Result<(), ExecutorError> {
        trace!("ElasticsearchExecutor - received action: \n[{:?}]", action);

        let data = action.payload.get(DATA_KEY).ok_or_else(|| {
            ExecutorError::MissingArgumentError { message: "data field is missing".to_string() }
        })?;

        let index_name =
            action.payload.get(INDEX_KEY).and_then(|val| val.get_text()).ok_or_else(|| {
                ExecutorError::MissingArgumentError {
                    message: "index field is missing".to_string(),
                }
            })?;

        let endpoint =
            action.payload.get(ENDPOINT_KEY).and_then(|val| val.get_text()).ok_or_else(|| {
                ExecutorError::MissingArgumentError {
                    message: "endpoint field is missing".to_string(),
                }
            })?;

        let endpoint =
            format!("{}/{}/_doc/", endpoint, utf8_percent_encode(index_name, NON_ALPHANUMERIC));

        let client = if let Some(auth) = action.payload.get(AUTH_KEY) {
            debug!("ElasticsearchExecutor - Found client data in payload. Create action specific client");
            let es_authentication: ElasticsearchAuthentication = serde_json::to_value(auth)
                .and_then(serde_json::from_value)
                .map_err(|err| ExecutorError::ActionExecutionError {
                    can_retry: false,
                    message: format!("Error while deserializing {}. Err: {:?}", AUTH_KEY, err),
                    code: None,
                    data: Default::default(),
                })?;
            Cow::Owned(es_authentication.new_client().await?)
        } else {
            debug!("ElasticsearchExecutor - Client data in payload not found. Use default client");
            Cow::Borrowed(self.default_client.as_ref().ok_or_else(|| {
                ExecutorError::ActionExecutionError {
                    can_retry: false,
                    message: "Missing both default client and auth data from payload".to_string(),
                    code: None,
                    data: Default::default(),
                }
            })?)
        };

        let res = client.post(&endpoint).json(&data).send().await.map_err(|err| {
            ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!("Error while sending document to Elasticsearch. Err: {:?}", err),
                code: None,
                data: Default::default(),
            }
        })?;

        if !res.status().is_success() {
            Err(ExecutorError::ActionExecutionError {
                can_retry: true,
                message: format!(
                    "Error while sending document to Elasticsearch. Response: {:?}",
                    res
                ),
                code: None,
                data: Default::default(),
            })
        } else {
            debug!("ElasticsearchExecutor - Data correctly sent to Elasticsearch");
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tornado_common_api::{Map, Value};

    // This can be used for local testing. It requires Elasticsearch running on localhost
    // #[tokio::test]
    // async fn should_send_document_to_elasticsearch() {
    //        // Arrange
    //        let es_authentication = Some(ElasticsearchAuthentication::PemCertificatePath {
    //            certificate_path: "/neteye/shared/tornado/conf/certs/tornado.crt.pem".to_string(),
    //            private_key_path: "/neteye/shared/tornado/conf/certs/private/tornado.key.pem"
    //                .to_string(),
    //            ca_certificate_path: "/neteye/shared/tornado/conf/certs/root-ca.crt".to_string(),
    //        });
    //        let executor = ElasticsearchExecutor::new(es_authentication).await.unwrap();
    //        let mut action = Action { id: "elasticsearch".to_string(), payload: Map::new() };
    //        let mut es_document = Map::new();
    //        es_document
    //            .insert("message".to_owned(), Value::String("message to elasticsearch".to_owned()));
    //        es_document.insert("user".to_owned(), Value::String("myuser".to_owned()));
    //        action.payload.insert("data".to_owned(), Value::Object(es_document));
    //        action.payload.insert("index".to_owned(), Value::String("tornado-example".to_owned()));
    //        action.payload.insert(
    //            "endpoint".to_owned(),
    //            Value::String("https://elasticsearch.neteyelocal:9200".to_owned()),
    //        );
    //
    //        // Act
    //        let result = executor.execute(action.into()).await;
    //
    //     // Assert
    //        assert!(result.is_ok());
    //    }

    // This can be used for local testing. It requires Elasticsearch running on localhost
    //    #[test]
    //    fn should_build_client_from_payload() {
    //        // Arrange
    //        let es_authentication = Some(ElasticsearchAuthentication::None {});
    //        let mut executor = ElasticsearchExecutor::new(es_authentication).unwrap();
    //        let mut action = Action { id: "elasticsearch".to_string(), payload: Map::new() };
    //        let mut es_document = Map::new();
    //        es_document
    //            .insert("message".to_owned(), Value::String("message to elasticsearch".to_owned()));
    //        es_document.insert("user".to_owned(), Value::String("myuser".to_owned()));
    //        action.payload.insert("data".to_owned(), Value::Object(es_document));
    //        action.payload.insert("index".to_owned(), Value::String("tornado-example".to_owned()));
    //        action.payload.insert(
    //            "endpoint".to_owned(),
    //            Value::String("https://elasticsearch.neteyelocal:9200".to_owned()),
    //        );
    //
    //        let mut auth = Map::new();
    //        auth.insert("type".to_owned(), Value::String("PemCertificatePath".to_owned()));
    //        auth.insert(
    //            "certificate_path".to_owned(),
    //            Value::String("/neteye/shared/tornado/conf/certs/tornado.crt.pem".to_owned()),
    //        );
    //        auth.insert(
    //            "private_key_path".to_owned(),
    //            Value::String("/neteye/shared/tornado/conf/certs/private/tornado.key.pem".to_owned()),
    //        );
    //        auth.insert(
    //            "ca_certificate_path".to_owned(),
    //            Value::String("/neteye/shared/tornado/conf/certs/root-ca.crt".to_owned()),
    //        );
    //        action.payload.insert("auth".to_owned(), Value::Object(auth));
    //
    //        // Act
    //        let result = executor.execute(action);
    //        result.unwrap();
    //        // Assert
    //        //            assert!(result.is_ok());
    //    }

    // This can be used for local testing. It requires Elasticsearch running on localhost
    // #[tokio::test]
    // async fn should_build_client_from_payload() {
    //     // Arrange
    //     let es_authentication = Some(ElasticsearchAuthentication::None {});
    //     let executor = ElasticsearchExecutor::new(es_authentication).await.unwrap();
    //     let mut action = Action { id: "elasticsearch".to_string(), payload: Map::new() };
    //     let mut es_document = Map::new();
    //     es_document
    //         .insert("message".to_owned(), Value::String("message to elasticsearch".to_owned()));
    //     es_document.insert("user".to_owned(), Value::String("myuser".to_owned()));
    //     action.payload.insert("data".to_owned(), Value::Object(es_document));
    //     action.payload.insert("index".to_owned(), Value::String("tornado-example".to_owned()));
    //     action.payload.insert(
    //         "endpoint".to_owned(),
    //         Value::String("http://localhost:9200".to_owned()),
    //     );
    //
    //     let mut action = Action { id: "elasticsearch".to_string(), payload: Map::new() };
    //     let mut es_document = Map::new();
    //     es_document
    //         .insert("message".to_owned(), Value::String("message to elasticsearch".to_owned()));
    //     es_document.insert("user".to_owned(), Value::String("myuser".to_owned()));
    //     action.payload.insert("data".to_owned(), Value::Object(es_document));
    //     action.payload.insert("index".to_owned(), Value::String("tornado-example".to_owned()));
    //     action.payload.insert(
    //         "endpoint".to_owned(),
    //         Value::String("http://localhost:9200".to_owned()),
    //     );
    //
    //     // Act
    //     let result = executor.execute(action.into()).await;
    //
    //     // Assert
    //     assert!(result.is_ok());
    // }

    #[tokio::test]
    async fn should_fail_if_index_is_missing() {
        // Arrange
        let executor = ElasticsearchExecutor::new(None).await.unwrap();
        let mut action = Action { id: "elasticsearch".to_string(), payload: Map::new() };
        let mut es_document = Map::new();
        es_document
            .insert("message".to_owned(), Value::String("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::String("myuser".to_owned()));

        action.payload.insert("data".to_owned(), Value::Object(es_document));
        action
            .payload
            .insert("endpoint".to_owned(), Value::String("http://127.0.0.1:9200".to_owned()));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        match result {
            Err(ExecutorError::MissingArgumentError { .. }) => {}
            _ => assert!(false),
        };
    }

    #[tokio::test]
    async fn should_fail_if_endpoint_is_missing() {
        // Arrange
        let executor = ElasticsearchExecutor::new(None).await.unwrap();
        let mut action = Action { id: "elasticsearch".to_string(), payload: Map::new() };
        let mut es_document = Map::new();
        es_document
            .insert("message".to_owned(), Value::String("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::String("myuser".to_owned()));

        action.payload.insert("data".to_owned(), Value::Object(es_document));
        action.payload.insert("index".to_owned(), Value::String("tornàdo".to_owned()));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        match result {
            Err(ExecutorError::MissingArgumentError { .. }) => {}
            _ => assert!(false),
        };
    }

    #[tokio::test]
    async fn should_fail_if_data_is_missing() {
        // Arrange
        let executor = ElasticsearchExecutor::new(None).await.unwrap();
        let mut action = Action { id: "elasticsearch".to_string(), payload: Map::new() };
        let mut es_document = Map::new();
        es_document
            .insert("message".to_owned(), Value::String("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::String("myuser".to_owned()));

        action
            .payload
            .insert("endpoint".to_owned(), Value::String("http://127.0.0.1:9200".to_owned()));
        action.payload.insert("index".to_owned(), Value::String("tornàdo".to_owned()));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        match result {
            Err(ExecutorError::MissingArgumentError { .. }) => {}
            _ => assert!(false),
        };
    }

    #[tokio::test]
    async fn should_fail_if_index_is_not_text() {
        // Arrange
        let executor = ElasticsearchExecutor::new(None).await.unwrap();
        let mut action = Action { id: "elasticsearch".to_string(), payload: Map::new() };
        let mut es_document = Map::new();
        es_document
            .insert("message".to_owned(), Value::String("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::String("myuser".to_owned()));

        action.payload.insert("data".to_owned(), Value::Object(es_document));
        action.payload.insert("index".to_owned(), Value::Array(vec![]));
        action
            .payload
            .insert("endpoint".to_owned(), Value::String("http://127.0.0.1:9200".to_owned()));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        match result {
            Err(ExecutorError::MissingArgumentError { .. }) => {}
            _ => assert!(false),
        };
    }

    #[tokio::test]
    async fn should_fail_if_endpoint_is_not_text() {
        // Arrange
        let executor = ElasticsearchExecutor::new(None).await.unwrap();
        let mut action = Action { id: "elasticsearch".to_string(), payload: Map::new() };
        let mut es_document = Map::new();
        es_document
            .insert("message".to_owned(), Value::String("message to elasticsearch".to_owned()));
        es_document.insert("user".to_owned(), Value::String("myuser".to_owned()));

        action.payload.insert("data".to_owned(), Value::Object(es_document));
        action.payload.insert("index".to_owned(), Value::String("tornàdo".to_owned()));
        action.payload.insert("endpoint".to_owned(), Value::Bool(false));

        // Act
        let result = executor.execute(action.into()).await;

        // Assert
        match result {
            Err(ExecutorError::MissingArgumentError { .. }) => {}
            _ => assert!(false),
        };
    }
}
