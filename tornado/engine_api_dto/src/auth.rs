use serde::{Deserialize, Serialize};
use typescript_definitions::TypeScriptify;

#[derive(Debug, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct Auth {
    pub user: String,
    pub roles: Vec<String>
}