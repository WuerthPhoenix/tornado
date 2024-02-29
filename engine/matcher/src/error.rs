use thiserror::Error;
use tornado_common_parser::ParserError;

#[derive(Error, Clone, Debug, PartialEq)]
pub enum MatcherError {
    #[error("ExtractorBuildFailError: [{message}]\n cause: [{cause}]")]
    ExtractorBuildFailError { message: String, cause: String },

    #[error("MissingExtractedVariableError: Cannot extract variable [{variable_name}].")]
    MissingExtractedVariableError { variable_name: String },

    #[error("ExtractedVariableError: Error extracting variable [{variable_name}]: {message}.")]
    ExtractedVariableError { variable_name: String, message: String },

    #[error("MissingOperatorError: No operator specified (the args array is empty)")]
    MissingOperatorError {},

    #[error("ParseOperatorError: [{message}]")]
    ParseOperatorError { message: String },

    #[error("UnknownOperatorError: Operator [{operator}] is unknown")]
    UnknownOperatorError { operator: String },

    #[error(
    "WrongNumberOfArgumentsError: While building operator [{operator}], expected arguments [{expected}], found [{found}]"
    )]
    WrongNumberOfArgumentsError { operator: &'static str, expected: u64, found: u64 },

    #[error("OperatorBuildFailError: [{message}]\n cause: [{cause}]")]
    OperatorBuildFailError { message: String, cause: String },

    #[error("UnknownAccessorError: Unknown accessor: [{accessor}]")]
    UnknownAccessorError { accessor: String },

    #[error("JsonDeserializationError: [{message}]")]
    JsonDeserializationError { message: String },

    #[error("ConfigurationError: [{message}]")]
    ConfigurationError { message: String },

    #[error("NotUniqueNameError: Two or more Nodes or Rules have the same name [{name}] but it must be unique."
    )]
    NotUniqueNameError { name: String },

    #[error("NotValidIdOrNameError: {message}")]
    NotValidIdOrNameError { message: String },

    #[error("CreateActionError: Cannot create action [{action_id}] for rule [{rule_name}]\n cause: [{cause}]"
    )]
    CreateActionError { action_id: String, rule_name: String, cause: String },

    #[error("StringInterpolatorRenderError: Cannot resolve placeholders in template [{template}] for rule [{rule_name}]\n cause: [{cause}]"
    )]
    InterpolatorRenderError { template: String, rule_name: String, cause: String },

    #[error("InternalSystemError: [{message}]")]
    InternalSystemError { message: String },
}

impl From<ParserError> for MatcherError {
    fn from(parser_error: ParserError) -> Self {
        MatcherError::ConfigurationError { message: format!("{:?}", parser_error) }
    }
}
