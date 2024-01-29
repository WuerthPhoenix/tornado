use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Error {
    kind: ErrorKind,
    span: Span,
}

#[wasm_bindgen]
impl Error {
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        format!("{:?}", self.kind)
    }

    #[wasm_bindgen(getter)]
    pub fn span(&self) -> Span {
        self.span
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    CaptureLimitExceeded,
    ClassEscapeInvalid,
    ClassRangeInvalid,
    ClassRangeLiteral,
    ClassUnclosed,
    DecimalEmpty,
    DecimalInvalid,
    EscapeHexEmpty,
    EscapeHexInvalid,
    EscapeHexInvalidDigit,
    EscapeUnexpectedEof,
    EscapeUnrecognized,
    FlagDanglingNegation,
    FlagDuplicate,
    FlagRepeatedNegation,
    FlagUnexpectedEof,
    FlagUnrecognized,
    GroupNameDuplicate,
    GroupNameEmpty,
    GroupNameInvalid,
    GroupNameUnexpectedEof,
    GroupUnclosed,
    GroupUnopened,
    NestLimitExceeded,
    RepetitionCountInvalid,
    RepetitionCountDecimalEmpty,
    RepetitionCountUnclosed,
    RepetitionMissing,
    UnicodeClassInvalid,
    UnsupportedBackreference,
    UnsupportedLookAround,
    UnicodeNotAllowed,
    InvalidUtf8,
    InvalidLineTerminator,
    UnicodePropertyNotFound,
    UnicodePropertyValueNotFound,
    UnicodePerlClassNotFound,
    UnicodeCaseUnavailable,
}

#[wasm_bindgen]
#[derive(Copy, Clone)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

#[wasm_bindgen]
#[derive(Copy, Clone)]
pub struct Position {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

impl From<regex_syntax::ast::Error> for Error {
    fn from(value: regex_syntax::ast::Error) -> Self {
        let kind = match value.kind() {
            regex_syntax::ast::ErrorKind::CaptureLimitExceeded => ErrorKind::CaptureLimitExceeded,
            regex_syntax::ast::ErrorKind::ClassEscapeInvalid => ErrorKind::ClassEscapeInvalid,
            regex_syntax::ast::ErrorKind::ClassRangeInvalid => ErrorKind::ClassRangeInvalid,
            regex_syntax::ast::ErrorKind::ClassRangeLiteral => ErrorKind::ClassRangeLiteral,
            regex_syntax::ast::ErrorKind::ClassUnclosed => ErrorKind::ClassUnclosed,
            regex_syntax::ast::ErrorKind::DecimalEmpty => ErrorKind::DecimalEmpty,
            regex_syntax::ast::ErrorKind::DecimalInvalid => ErrorKind::DecimalInvalid,
            regex_syntax::ast::ErrorKind::EscapeHexEmpty => ErrorKind::EscapeHexEmpty,
            regex_syntax::ast::ErrorKind::EscapeHexInvalid => ErrorKind::EscapeHexInvalid,
            regex_syntax::ast::ErrorKind::EscapeHexInvalidDigit => ErrorKind::EscapeHexInvalidDigit,
            regex_syntax::ast::ErrorKind::EscapeUnexpectedEof => ErrorKind::EscapeUnexpectedEof,
            regex_syntax::ast::ErrorKind::EscapeUnrecognized => ErrorKind::EscapeUnrecognized,
            regex_syntax::ast::ErrorKind::FlagDanglingNegation => ErrorKind::FlagDanglingNegation,
            regex_syntax::ast::ErrorKind::FlagDuplicate { .. } => ErrorKind::FlagDuplicate,
            regex_syntax::ast::ErrorKind::FlagRepeatedNegation { .. } => {
                ErrorKind::FlagRepeatedNegation
            }
            regex_syntax::ast::ErrorKind::FlagUnexpectedEof => ErrorKind::FlagUnexpectedEof,
            regex_syntax::ast::ErrorKind::FlagUnrecognized => ErrorKind::FlagUnrecognized,
            regex_syntax::ast::ErrorKind::GroupNameDuplicate { .. } => {
                ErrorKind::GroupNameDuplicate
            }
            regex_syntax::ast::ErrorKind::GroupNameEmpty => ErrorKind::GroupNameEmpty,
            regex_syntax::ast::ErrorKind::GroupNameInvalid => ErrorKind::GroupNameInvalid,
            regex_syntax::ast::ErrorKind::GroupNameUnexpectedEof => {
                ErrorKind::GroupNameUnexpectedEof
            }
            regex_syntax::ast::ErrorKind::GroupUnclosed => ErrorKind::GroupUnclosed,
            regex_syntax::ast::ErrorKind::GroupUnopened => ErrorKind::GroupUnopened,
            regex_syntax::ast::ErrorKind::NestLimitExceeded(_) => ErrorKind::NestLimitExceeded,
            regex_syntax::ast::ErrorKind::RepetitionCountInvalid => {
                ErrorKind::RepetitionCountInvalid
            }
            regex_syntax::ast::ErrorKind::RepetitionCountDecimalEmpty => {
                ErrorKind::RepetitionCountDecimalEmpty
            }
            regex_syntax::ast::ErrorKind::RepetitionCountUnclosed => {
                ErrorKind::RepetitionCountUnclosed
            }
            regex_syntax::ast::ErrorKind::RepetitionMissing => ErrorKind::RepetitionMissing,
            regex_syntax::ast::ErrorKind::UnicodeClassInvalid => ErrorKind::UnicodeClassInvalid,
            regex_syntax::ast::ErrorKind::UnsupportedBackreference => {
                ErrorKind::UnsupportedBackreference
            }
            regex_syntax::ast::ErrorKind::UnsupportedLookAround => ErrorKind::UnsupportedLookAround,
            _ => panic!(),
        };

        let span = Span {
            start: Position {
                offset: value.span().start.offset,
                line: value.span().start.line,
                column: value.span().start.column,
            },
            end: Position {
                offset: value.span().end.offset,
                line: value.span().end.line,
                column: value.span().end.column,
            },
        };

        Self { kind, span }
    }
}

impl From<regex_syntax::hir::Error> for Error {
    fn from(value: regex_syntax::hir::Error) -> Self {
        let kind = match value.kind() {
            regex_syntax::hir::ErrorKind::UnicodeNotAllowed => ErrorKind::UnicodeNotAllowed,
            regex_syntax::hir::ErrorKind::InvalidUtf8 => ErrorKind::InvalidUtf8,
            regex_syntax::hir::ErrorKind::InvalidLineTerminator => ErrorKind::InvalidLineTerminator,
            regex_syntax::hir::ErrorKind::UnicodePropertyNotFound => {
                ErrorKind::UnicodePropertyNotFound
            }
            regex_syntax::hir::ErrorKind::UnicodePropertyValueNotFound => {
                ErrorKind::UnicodePropertyValueNotFound
            }
            regex_syntax::hir::ErrorKind::UnicodePerlClassNotFound => {
                ErrorKind::UnicodePerlClassNotFound
            }
            regex_syntax::hir::ErrorKind::UnicodeCaseUnavailable => {
                ErrorKind::UnicodeCaseUnavailable
            }
            _ => panic!(),
        };

        let span = Span {
            start: Position {
                offset: value.span().start.offset,
                line: value.span().start.line,
                column: value.span().start.column,
            },
            end: Position {
                offset: value.span().end.offset,
                line: value.span().end.line,
                column: value.span().end.column,
            },
        };

        Self { kind, span }
    }
}
