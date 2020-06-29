use serde::{Deserialize, Serialize};
use typescript_definitions::TypeScriptify;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct UserPreferences {
    pub language: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct Auth {
    pub user: String,
    pub roles: Vec<String>,
    pub preferences: Option<UserPreferences>,
}

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, Serialize, Deserialize, TypeScriptify)]
pub enum PermissionDto {
    ConfigEdit,
    ConfigView,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct AuthWithPermissionsDto {
    pub user: String,
    pub permissions: Vec<PermissionDto>,
    pub preferences: Option<UserPreferences>,
}

impl Auth {
    pub fn new<S: Into<String>, R: Into<String>>(user: S, roles: Vec<R>) -> Self {
        Auth {
            user: user.into(),
            preferences: None,
            roles: roles.into_iter().map(|role| role.into()).collect(),
        }
    }
}
