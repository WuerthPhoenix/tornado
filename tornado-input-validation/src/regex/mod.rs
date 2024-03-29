use crate::regex::error::RegexError;
use regex_syntax::hir::{Hir, HirKind};
use wasm_bindgen::prelude::*;

mod error;

#[wasm_bindgen]
pub struct RegexValidationResult {
    pub is_valid: bool,
    pub has_named_groups: bool,
    pub error: Option<RegexError>,
}

#[wasm_bindgen]
pub fn validate_regex(reg_exp: &str) -> RegexValidationResult {
    match regex_syntax::parse(reg_exp) {
        Ok(hir) => RegexValidationResult {
            is_valid: true,
            has_named_groups: find_named_group(&hir),
            error: None,
        },
        Err(regex_syntax::Error::Parse(e)) => RegexValidationResult {
            is_valid: false,
            has_named_groups: false,
            error: Some(e.into()),
        },
        Err(regex_syntax::Error::Translate(e)) => RegexValidationResult {
            is_valid: false,
            has_named_groups: false,
            error: Some(e.into()),
        },
        Err(_) => RegexValidationResult { is_valid: false, has_named_groups: false, error: None },
    }
}

fn find_named_group(hir: &Hir) -> bool {
    match hir.kind() {
        HirKind::Empty | HirKind::Literal(_) | HirKind::Class(_) | HirKind::Look(_) => false,
        HirKind::Repetition(rep) => find_named_group(&rep.sub),
        HirKind::Capture(capture) => capture.name.is_some() || find_named_group(&capture.sub),
        HirKind::Concat(vec) | HirKind::Alternation(vec) => vec.iter().any(find_named_group),
    }
}
