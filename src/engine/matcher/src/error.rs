#[derive(Fail, Debug)]
pub enum MatcherError {
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
}
