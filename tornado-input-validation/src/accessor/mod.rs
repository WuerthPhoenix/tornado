use crate::accessor::error::AccessorError;
use tornado_common_parser::{Parser, ParserBuilder};
use wasm_bindgen::prelude::wasm_bindgen;

mod error;

#[wasm_bindgen]
pub struct AccessorValidationResult {
    pub is_valid: bool,
    pub r#type: AccessorType,
    error: Option<AccessorError>,
}

#[wasm_bindgen]
pub enum AccessorType {
    None,
    Expression,
    StringInterpolator,
    Static,
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
    let result =
        AccessorValidationResult { is_valid: true, r#type: AccessorType::Expression, error: None };
    match ParserBuilder::engine_matcher(input) {
        Ok(Parser::Exp(_)) | Ok(Parser::Custom { .. }) => result,
        Ok(Parser::Interpolator { .. }) => {
            AccessorValidationResult { r#type: AccessorType::StringInterpolator, ..result }
        }
        Ok(Parser::Val(_)) => AccessorValidationResult { r#type: AccessorType::Static, ..result },
        Err(error) => AccessorValidationResult {
            is_valid: false,
            r#type: AccessorType::None,
            error: Some(error.into()),
        },
    }
}
