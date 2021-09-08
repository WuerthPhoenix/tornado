use serde::{Deserialize, Serialize};
use tornado_executor_common::ExecutorError;
use std::io::BufReader;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::rustls::internal::pemfile;

use log::{info, debug};
use tokio_rustls::{webpki, TlsConnector};
use std::sync::Arc;
use tokio::net::TcpStream;
use crate::connection::Connection;
use std::fs::File;

const CERT_DIR: &str = "/neteye/local/icinga2/data/lib/icinga2/certs";

#[derive(Deserialize, Serialize, Clone)]
pub struct Icinga2ClientConfig {
    /// The complete URL of the API Server
    pub server_api_url: String,

    /// The port on which the icinga2-master listens to
    pub port: u16,

    /// Common name for the icinga2 endpoint used to connect to the APIs
    pub cn: String,

    /// The common name of the master node to connect to
    pub master_cn: String,
}

impl Icinga2ClientConfig {
    /// tries to establish a connection with the given parameters. Returns a Result with
    /// the connection if it was successful or an ExecutorError if a problem has arisen
    pub async fn connect(&self) -> Result<Connection, ExecutorError> {
        let address = format!("{}:{}", self.server_api_url, self.port);

        let mut config = ClientConfig::new();
        Self::load_master_ca(&mut config)?;
        self.load_client_cert(&mut config)?;

        let connector: TlsConnector = Arc::new(config).into();
        let com = webpki::DNSNameRef::try_from_ascii_str("icinga2-master.neteyelocal").unwrap();

        info!("Icinga2Connector - Trying to connect to {} as {}", address, self.cn);

        let stream = TcpStream::connect(&address).await.unwrap();
        let stream = connector.connect(com, stream).await.unwrap();

        info!("Icinga2Connector - Connection to [{}] established", address);

        let stream = tokio::io::BufReader::new(stream);

        Ok(Connection::from(stream).await.unwrap())
    }

    fn load_master_ca(config: &mut ClientConfig) -> Result<(), ExecutorError> {
        let cert_path = format!("{}/ca.crt", CERT_DIR);

        let file = std::fs::File::open(&cert_path).map_err(|err| {
            ExecutorError::ConfigurationError {
                message: format!(
                    "Icinga2Common - Could not load icinga2-master root certificate from {}. Err: {:?}",
                    cert_path,
                    err
                ),
            }
        })?;

        let mut reader = BufReader::new(file);
        config.root_store.add_pem_file(&mut reader).map(|_| ()).map_err( |err|
            ExecutorError::ConfigurationError {
                message: format!(
                    "Icinga2Common - Could not read icinga2-master root certificate. Err: {:?}",
                    err
                ),
            }
        )?;

        Ok(())
    }

    fn load_client_cert(&self, config: &mut ClientConfig) -> Result<(), ExecutorError> {
        let cert_path = format!("{}/{}.crt", CERT_DIR, self.cn);
        let key_path = format!("{}/{}.key", CERT_DIR, self.cn);

        debug!("Icinga2Common - Loading certificate from {}", cert_path);

        let file = match File::open(&cert_path) {
            Ok(file) => file,
            Err(err) => return Err(ExecutorError::ConfigurationError {
                message: format!("Icinga2Common - Could not open file {}. {}", cert_path, err)
            })
        };
        let crt = match pemfile::certs(&mut BufReader::new(file)) {
            Ok(crt) => crt,
            Err(_) => return Err(ExecutorError::ConfigurationError {
                message: format!("Icinga2Common - Could not load client certificate from {}", cert_path)
            }),
        };

        let file = match File::open(&key_path){
            Ok(file) => file,
            Err(err) => return Err(ExecutorError::ConfigurationError {
                message: format!("Icinga2Common - Could not open file {}. {}", cert_path, err)
            })
        };
        let key = match pemfile::rsa_private_keys(&mut BufReader::new(file)) {
            Ok(key) if key.is_empty() => return Err(ExecutorError::ConfigurationError {
                message: format!("Icinga2Common - No private key found in file {}", key_path)
            }),
            Ok(key) => key[0].clone(),
            Err(_) => return Err(ExecutorError::ConfigurationError {
                message: format!("Icinga2Common - Could not load client private key from {}", key_path)
            })
        };

        match config.set_single_client_cert(crt, key) {
            Ok(()) => Ok(()),
            Err(_) => Err(ExecutorError::ConfigurationError {
                message: "Icinga2Common - Could not set single client certificate".to_owned()
            }),
        }
    }
}
