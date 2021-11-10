use crate::auth::UserPreferences;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typescript_definitions::TypeScriptify;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct AuthHeaderV2 {
    pub user: String,
    pub auths: HashMap<String, Authorization>,
    pub preferences: Option<UserPreferences>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct AuthV2 {
    pub user: String,
    pub authorization: Authorization,
    pub preferences: Option<UserPreferences>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct Authorization {
    pub path: Vec<String>,
    pub roles: Vec<String>,
}
