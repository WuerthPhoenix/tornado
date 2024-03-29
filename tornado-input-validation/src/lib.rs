#![allow(clippy::enum_variant_names)]

mod accessor;
mod regex;

pub use accessor::{validate_accessor, AccessorValidationResult};
pub use regex::{validate_regex, RegexValidationResult};
