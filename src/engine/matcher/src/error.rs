#[derive(Fail, Debug)]
pub enum MatcherError {
    #[fail(
        display = "ExtractorBuildFailError: [{}]\n cause: [{}]",
        message,
        cause
    )]
    ExtractorBuildFailError { message: String, cause: String },

    #[fail(
        display = "MissingExtractedVariableError: Extractor cannot extract variable [{}].",
        variable_name
    )]
    MissingExtractedVariableError { variable_name: String },

    #[fail(display = "MissingOperatorError: No operator specified (the args array is empty)")]
    MissingOperatorError {},

    #[fail(display = "ParseOperatorError: [{}]", message)]
    ParseOperatorError { message: String },

    #[fail(
        display = "UnknownOperatorError: Operator [{}] is unknown",
        operator
    )]
    UnknownOperatorError { operator: String },

    #[fail(
        display = "WrongNumberOfArgumentsError: While building operator [{}], expected arguments [{}], found [{}]",
        operator,
        expected,
        found
    )]
    WrongNumberOfArgumentsError {
        operator: &'static str,
        expected: u64,
        found: u64,
    },

    #[fail(
        display = "OperatorBuildFailError: [{}]\n cause: [{}]",
        message,
        cause
    )]
    OperatorBuildFailError { message: String, cause: String },

    #[fail(
        display = "UnknownAccessorError: Unknown accessor: [{}]",
        accessor
    )]
    UnknownAccessorError { accessor: String },

    #[fail(display = "AccessorWrongPayloadKeyError: [{}]", payload_key)]
    AccessorWrongPayloadKeyError { payload_key: String },

    #[fail(display = "JsonDeserializationError: [{}]", message)]
    JsonDeserializationError { message: String },

    #[fail(
        display = "NotUniqueRulePriorityError: The Rule [{}] and [{}] have the same priority [{}] but it must be unique.",
        first_rule_name,
        second_rule_name,
        priority
    )]
    NotUniqueRulePriorityError {
        first_rule_name: String,
        second_rule_name: String,
        priority: u16,
    },

    #[fail(
        display = "NotUniqueRuleNameError: Two or more Rules have the same name [{}] but it must be unique.",
        name
    )]
    NotUniqueRuleNameError { name: String },
}
