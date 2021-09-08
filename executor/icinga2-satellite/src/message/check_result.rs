use either::Either;
use either::Either::{Left, Right};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::message::timestamp::TimeStamp;

/// The CheckResult is the core concept of the icinga2 protocol. It is sent to update the master
/// about the state of a certain checkable object. The CheckResult has to have three values in order
/// to be correctly processed by the master:
///
///     * The type of the CheckResult for the purposes of a satellite which sends passive CheckResults,
///     is always "CheckResult".
///
///     * The state is the core part of the message and needs to be a integer between 1 and 4. However
///     because of the Json specification which has no Integer defined, it is usually represented by
///     icinga2 as a floating point number, with a zero as mantissa.
///
///     * The last object that needs to be present is the performance data. It is a list of Strings
///     which holds data to the check. This value can be empty ([]) as long as the key is present.
///
/// Furthermore it can hold a bunch of other values to provide further information about the test.
/// Those are however all optional.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct CheckResult {
    #[serde(rename = "type", default = "default_object_type")]
    object_type: String,
    #[serde(default = "default_object_state")]
    state: f64,
    #[serde(default = "Vec::new")]
    pub performance_data: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_status: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Command>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_start: Option<TimeStamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_end: Option<TimeStamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_start: Option<TimeStamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_end: Option<TimeStamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vars_before: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vars_after: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<f64>,
}

impl CheckResult {
    pub fn for_state(state: u32) -> CheckResult {
        CheckResult {
            object_type: "CheckResult".to_string(),
            performance_data: vec![],
            state: state as f64,
            output: None,
            exit_status: None,
            check_source: None,
            command: None,
            execution_start: None,
            execution_end: None,
            schedule_start: None,
            schedule_end: None,
            active: None,
            vars_before: None,
            vars_after: None,
            ttl: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Command(
    #[serde(with = "either::serde_untagged")]
    Either<String, Vec<String>>
);

impl From<String> for Command {
    fn from(command: String) -> Self {
        Command(Left(command))
    }
}

impl From<Vec<String>> for Command {
    fn from(command: Vec<String>) -> Self {
        Command(Right(command))
    }
}

impl Default for CheckResult {
    fn default() -> Self {
        CheckResult::for_state(0)
    }
}

fn default_object_type() -> String {
    "CheckResult".to_owned()
}

fn default_object_state() -> f64 {
    0.0
}

#[cfg(test)]
mod test {
    use super::CheckResult;

    #[test]
    fn should_serialize_with_command() {
        let mut ser = CheckResult::for_state(0);
        ser.command = Some("test_command".to_string().into());

        let expected = r#"{"type":"CheckResult","state":0.0,"performance_data":[],"command":"test_command"}"#;

        assert_eq!(expected, serde_json::to_string(&ser).unwrap());
    }
}