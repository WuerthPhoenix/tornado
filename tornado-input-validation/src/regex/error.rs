use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct RegexError {
    kind: RegexErrorKind,
    span: Span,
}

#[wasm_bindgen]
impl RegexError {
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
pub enum RegexErrorKind {
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

impl From<regex_syntax::ast::Error> for RegexError {
    fn from(value: regex_syntax::ast::Error) -> Self {
        let kind = match value.kind() {
            regex_syntax::ast::ErrorKind::CaptureLimitExceeded => {
                RegexErrorKind::CaptureLimitExceeded
            }
            regex_syntax::ast::ErrorKind::ClassEscapeInvalid => RegexErrorKind::ClassEscapeInvalid,
            regex_syntax::ast::ErrorKind::ClassRangeInvalid => RegexErrorKind::ClassRangeInvalid,
            regex_syntax::ast::ErrorKind::ClassRangeLiteral => RegexErrorKind::ClassRangeLiteral,
            regex_syntax::ast::ErrorKind::ClassUnclosed => RegexErrorKind::ClassUnclosed,
            regex_syntax::ast::ErrorKind::DecimalEmpty => RegexErrorKind::DecimalEmpty,
            regex_syntax::ast::ErrorKind::DecimalInvalid => RegexErrorKind::DecimalInvalid,
            regex_syntax::ast::ErrorKind::EscapeHexEmpty => RegexErrorKind::EscapeHexEmpty,
            regex_syntax::ast::ErrorKind::EscapeHexInvalid => RegexErrorKind::EscapeHexInvalid,
            regex_syntax::ast::ErrorKind::EscapeHexInvalidDigit => {
                RegexErrorKind::EscapeHexInvalidDigit
            }
            regex_syntax::ast::ErrorKind::EscapeUnexpectedEof => {
                RegexErrorKind::EscapeUnexpectedEof
            }
            regex_syntax::ast::ErrorKind::EscapeUnrecognized => RegexErrorKind::EscapeUnrecognized,
            regex_syntax::ast::ErrorKind::FlagDanglingNegation => {
                RegexErrorKind::FlagDanglingNegation
            }
            regex_syntax::ast::ErrorKind::FlagDuplicate { .. } => RegexErrorKind::FlagDuplicate,
            regex_syntax::ast::ErrorKind::FlagRepeatedNegation { .. } => {
                RegexErrorKind::FlagRepeatedNegation
            }
            regex_syntax::ast::ErrorKind::FlagUnexpectedEof => RegexErrorKind::FlagUnexpectedEof,
            regex_syntax::ast::ErrorKind::FlagUnrecognized => RegexErrorKind::FlagUnrecognized,
            regex_syntax::ast::ErrorKind::GroupNameDuplicate { .. } => {
                RegexErrorKind::GroupNameDuplicate
            }
            regex_syntax::ast::ErrorKind::GroupNameEmpty => RegexErrorKind::GroupNameEmpty,
            regex_syntax::ast::ErrorKind::GroupNameInvalid => RegexErrorKind::GroupNameInvalid,
            regex_syntax::ast::ErrorKind::GroupNameUnexpectedEof => {
                RegexErrorKind::GroupNameUnexpectedEof
            }
            regex_syntax::ast::ErrorKind::GroupUnclosed => RegexErrorKind::GroupUnclosed,
            regex_syntax::ast::ErrorKind::GroupUnopened => RegexErrorKind::GroupUnopened,
            regex_syntax::ast::ErrorKind::NestLimitExceeded(_) => RegexErrorKind::NestLimitExceeded,
            regex_syntax::ast::ErrorKind::RepetitionCountInvalid => {
                RegexErrorKind::RepetitionCountInvalid
            }
            regex_syntax::ast::ErrorKind::RepetitionCountDecimalEmpty => {
                RegexErrorKind::RepetitionCountDecimalEmpty
            }
            regex_syntax::ast::ErrorKind::RepetitionCountUnclosed => {
                RegexErrorKind::RepetitionCountUnclosed
            }
            regex_syntax::ast::ErrorKind::RepetitionMissing => RegexErrorKind::RepetitionMissing,
            regex_syntax::ast::ErrorKind::UnicodeClassInvalid => {
                RegexErrorKind::UnicodeClassInvalid
            }
            regex_syntax::ast::ErrorKind::UnsupportedBackreference => {
                RegexErrorKind::UnsupportedBackreference
            }
            regex_syntax::ast::ErrorKind::UnsupportedLookAround => {
                RegexErrorKind::UnsupportedLookAround
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

impl From<regex_syntax::hir::Error> for RegexError {
    fn from(value: regex_syntax::hir::Error) -> Self {
        let kind = match value.kind() {
            regex_syntax::hir::ErrorKind::UnicodeNotAllowed => RegexErrorKind::UnicodeNotAllowed,
            regex_syntax::hir::ErrorKind::InvalidUtf8 => RegexErrorKind::InvalidUtf8,
            regex_syntax::hir::ErrorKind::InvalidLineTerminator => {
                RegexErrorKind::InvalidLineTerminator
            }
            regex_syntax::hir::ErrorKind::UnicodePropertyNotFound => {
                RegexErrorKind::UnicodePropertyNotFound
            }
            regex_syntax::hir::ErrorKind::UnicodePropertyValueNotFound => {
                RegexErrorKind::UnicodePropertyValueNotFound
            }
            regex_syntax::hir::ErrorKind::UnicodePerlClassNotFound => {
                RegexErrorKind::UnicodePerlClassNotFound
            }
            regex_syntax::hir::ErrorKind::UnicodeCaseUnavailable => {
                RegexErrorKind::UnicodeCaseUnavailable
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
