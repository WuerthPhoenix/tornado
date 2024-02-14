use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub enum AccessorErrorKind {
    UnknownKeyError,
    NotANumberError,
    InvalidCharacterError,
    EmptyAccessorError,
}

// The AccessorError struct must be "inspectable" because otherwise the
// translation module in the UI cannot access its properties.
#[wasm_bindgen(inspectable)]
#[derive(Clone)]
pub struct AccessorError {
    kind: AccessorErrorKind,
    key: Option<String>,
    character: Option<String>,
}

#[wasm_bindgen]
impl AccessorError {
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        format!("{:?}", self.kind)
    }
    #[wasm_bindgen(getter)]
    pub fn key(&self) -> Option<String> {
        self.key.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn character(&self) -> Option<String> {
        self.character.clone()
    }
}

impl From<tornado_common_parser::ParserError> for AccessorError {
    fn from(value: tornado_common_parser::ParserError) -> Self {
        match value {
            tornado_common_parser::ParserError::UnknownKeyError { key } => AccessorError {
                kind: AccessorErrorKind::UnknownKeyError,
                key: Some(key),
                character: None,
            },
            tornado_common_parser::ParserError::NotANumberError { key } => AccessorError {
                kind: AccessorErrorKind::NotANumberError,
                key: Some(key),
                character: None,
            },
            tornado_common_parser::ParserError::InvalidCharacterError { key, character } => {
                AccessorError {
                    kind: AccessorErrorKind::InvalidCharacterError,
                    key: Some(key),
                    character: Some(character),
                }
            }
            tornado_common_parser::ParserError::EmptyAccessorError => AccessorError {
                kind: AccessorErrorKind::EmptyAccessorError,
                key: None,
                character: None,
            },
        }
    }
}
