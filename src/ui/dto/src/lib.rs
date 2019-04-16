#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};
use typescript_definitions::TypescriptDefinition;
use std::collections::HashMap;
use serde_json::Value;


#[derive(Clone, Serialize, Deserialize, TypescriptDefinition, Default)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub created_ms: u64,
    pub payload: HashMap<String, Value>,
}


#[derive(Deserialize, Serialize, TypescriptDefinition, Default, Clone)]
pub struct EventDto {
    pub event: Event,
}

/*
#[derive(Deserialize, Serialize, TypescriptDefinition, Default, Clone)]
pub struct MatcherConfigResponse {
    pub config: MatcherConfig,
}
*/