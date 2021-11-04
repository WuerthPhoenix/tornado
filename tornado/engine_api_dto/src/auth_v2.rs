use crate::auth::UserPreferences;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typescript_definitions::TypeScriptify;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct AuthHeaderV2 {
    pub user: String,
    pub auths: HashMap<String, AuthInstance>,
    pub preferences: Option<UserPreferences>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct AuthV2 {
    pub user: String,
    pub auth_instance: AuthInstance,
    pub preferences: Option<UserPreferences>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct AuthInstance {
    pub path: Vec<String>,
    pub roles: Vec<String>,
}
