use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct ValidationResult {
    is_valid: bool,
    error: Option<String>,
}

#[wasm_bindgen]
impl ValidationResult {
    #[wasm_bindgen(getter)]
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }
}

#[wasm_bindgen]
pub fn validate_regex(reg_exp: &str) -> ValidationResult {
    match regex_syntax::parse(reg_exp) {
        Ok(_) => ValidationResult { is_valid: true, error: None },
        Err(e) => ValidationResult { is_valid: false, error: Some(e.to_string()) },
    }
}
