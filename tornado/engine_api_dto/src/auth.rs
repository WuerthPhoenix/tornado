use serde::{Deserialize, Serialize};
use typescript_definitions::TypeScriptify;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct Auth {
    pub user: String,
    pub roles: Vec<String>,
}

impl Auth {
    pub fn new<S: Into<String>, R: Into<String>>(user: S, roles: Vec<R>) -> Self {
        Auth { user: user.into(), roles: roles.into_iter().map(|role| role.into()).collect() }
    }
}
