use crate::accessor::error::AccessorError;
use tornado_common_parser::ParserBuilder;
use wasm_bindgen::prelude::wasm_bindgen;

mod error;

#[wasm_bindgen]
pub struct AccessorValidationResult {
    pub is_valid: bool,
    error: Option<AccessorError>,
}

#[wasm_bindgen]
impl AccessorValidationResult {
    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<AccessorError> {
        self.error.clone()
    }
}

#[wasm_bindgen]
pub fn validate_accessor(input: &str) -> AccessorValidationResult {
    match ParserBuilder::engine_matcher(input) {
        Ok(_) => AccessorValidationResult { is_valid: true, error: None },
        Err(error) => AccessorValidationResult { is_valid: false, error: Some(error.into()) },
    }
}
