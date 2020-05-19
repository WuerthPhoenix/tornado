use serde::{Deserialize, Serialize};
use typescript_definitions::TypeScriptify;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct Id<T> {
    pub id: T,
}

#[derive(Serialize, TypeScriptify)]
pub struct WebError {
    pub code: String,
    pub message: Option<String>,
}
