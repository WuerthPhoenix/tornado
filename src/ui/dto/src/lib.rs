#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub mod config;
pub mod event;

// This static string will be injected into the TypeScript definition file.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
export type Value = any;
"#;
