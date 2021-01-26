use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Icinga2ClientConfig {
    /// The complete URL of the API Server
    pub server_api_url: String,

    /// Username used to connect to the APIs
    pub username: String,

    /// Password used to connect to the APIs
    pub password: String,

    /// If true, the client will not verify the SSL certificate
    pub disable_ssl_verification: bool,

    /// The call timeout in seconds. Default is 10 seconds
    pub timeout_secs: Option<u64>,
}
