use tornado_common_parser::{Parser, ParserBuilder, ParserError};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct AccessorValidationResult {
    pub is_valid: bool,
    error: Option<String>,
}

#[wasm_bindgen]
impl AccessorValidationResult {
    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }
}

impl From<Result<Parser<&str>, ParserError>> for AccessorValidationResult {
    fn from(value: Result<Parser<&str>, ParserError>) -> Self {
        match value {
            Ok(_) => AccessorValidationResult { is_valid: true, error: None },
            Err(err) => AccessorValidationResult { is_valid: false, error: Some(err.to_string()) },
        }
    }
}

#[wasm_bindgen]
pub fn validate_accessor(parser: &str) -> AccessorValidationResult {
    AccessorValidationResult::from(ParserBuilder::default().build_parser(parser))
}
