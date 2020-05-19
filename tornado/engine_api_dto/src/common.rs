use serde::{Deserialize, Serialize};
use typescript_definitions::TypeScriptify;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, TypeScriptify)]
pub struct Id<T> {
    pub id: T,
}

#[derive(Serialize, TypeScriptify)]
pub struct WebError {
    pub code: String,
    pub params: HashMap<String, String>,
    pub message: Option<String>,
}
