use failure_derive::Fail;

#[derive(Fail, Debug, PartialEq)]
pub enum MatcherError {
    #[fail(display = "ExtractorBuildFailError: [{}]\n cause: [{}]", message, cause)]
    ExtractorBuildFailError { message: String, cause: String },

    #[fail(
        display = "MissingExtractedVariableError: Cannot extract variable [{}].",
        variable_name
    )]
    MissingExtractedVariableError { variable_name: String },

    #[fail(display = "MissingOperatorError: No operator specified (the args array is empty)")]
    MissingOperatorError {},

    #[fail(display = "ParseOperatorError: [{}]", message)]
    ParseOperatorError { message: String },

    #[fail(display = "UnknownOperatorError: Operator [{}] is unknown", operator)]
    UnknownOperatorError { operator: String },

    #[fail(
        display = "WrongNumberOfArgumentsError: While building operator [{}], expected arguments [{}], found [{}]",
        operator, expected, found
    )]
    WrongNumberOfArgumentsError { operator: &'static str, expected: u64, found: u64 },

    #[fail(display = "OperatorBuildFailError: [{}]\n cause: [{}]", message, cause)]
    OperatorBuildFailError { message: String, cause: String },

    #[fail(display = "UnknownAccessorError: Unknown accessor: [{}]", accessor)]
    UnknownAccessorError { accessor: String },

    #[fail(display = "JsonDeserializationError: [{}]", message)]
    JsonDeserializationError { message: String },

    #[fail(display = "ConfigurationError: [{}]", message)]
    ConfigurationError { message: String },

    #[fail(
        display = "NotUniqueRuleNameError: Two or more Rules have the same name [{}] but it must be unique.",
        name
    )]
    NotUniqueRuleNameError { name: String },

    #[fail(display = "NotValidIdOrNameError: {}", message)]
    NotValidIdOrNameError { message: String },

    #[fail(
        display = "CreateActionError: Cannot create action [{}] for rule [{}]\n cause: [{}]",
        action_id, rule_name, cause
    )]
    CreateActionError { action_id: String, rule_name: String, cause: String },
}
