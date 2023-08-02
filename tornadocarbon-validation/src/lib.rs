use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn validate_regex(reg_exp: &str) -> bool {
    regex_syntax::parse(reg_exp).is_ok()
}
