#![allow(illegal_floating_point_literal_pattern)]

use crate::error::MatcherError;
use chrono::TimeZone;
use chrono_tz::Tz;
use serde_json::Value;

#[inline]
pub fn dateandtime(
    variable_name: &str,
    value: &mut Value,
    timezone: &Tz,
) -> Result<(), MatcherError> {
    let Some(timestamp) = value.as_i64() else {
        return Err(MatcherError::ExtractedVariableError {
            message: format!("The value passed to the dateandtime modifier is not valid (must be an integer): {}", value),
            variable_name: variable_name.to_owned(),
        });
    };

    // This is triggering a warning during the build.
    // It should be fixed automatically with a newer version of rust, for
    // more info: https://github.com/rust-lang/rust/issues/41620)
    let date = match timestamp as f64 {
        // timestamp is in seconds
        -1e11..=1e11 => timezone.timestamp_opt(timestamp, 0).unwrap(),
        // timestamp is in milliseconds
        -1e14..=1e14 => timezone.timestamp_millis_opt(timestamp).unwrap(),
        // timestamp is in microseconds
        -1e17..=1e17 => timezone.timestamp_nanos(timestamp * 1000),
        // timestamp is in nanoseconds
        _ => timezone.timestamp_nanos(timestamp),
    };

    *value = Value::String(date.format("%Y-%m-%d %H:%M:%S%:z").to_string());
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::Map;

    #[test]
    fn datetime_modifier_should_convert_unix_timestamp_seconds_to_datetime_string() {
        {
            let mut input = Value::from(1698915872);
            dateandtime("", &mut input, &Tz::Europe__London).unwrap();
            assert_eq!(Value::String("2023-11-02 09:04:32+00:00".to_owned()), input);
        }

        {
            let mut input = Value::from(1698915872);
            dateandtime("", &mut input, &Tz::Europe__Rome).unwrap();
            assert_eq!(Value::String("2023-11-02 10:04:32+01:00".to_owned()), input);
        }

        {
            let mut input = Value::from(1698915872);
            dateandtime("", &mut input, &Tz::America__New_York).unwrap();
            assert_eq!(Value::String("2023-11-02 05:04:32-04:00".to_owned()), input);
        }
    }

    #[test]
    fn datetime_modifier_should_convert_unix_timestamp_milliseconds_to_datetime_string() {
        {
            let timestamp: i64 = 1698933188760;
            let mut input = Value::from(timestamp);
            dateandtime("", &mut input, &Tz::Europe__London).unwrap();
            assert_eq!(Value::String("2023-11-02 13:53:08+00:00".to_owned()), input);
        }

        {
            let timestamp: i64 = 1698933188760;
            let mut input = Value::from(timestamp);
            dateandtime("", &mut input, &Tz::Europe__Rome).unwrap();
            assert_eq!(Value::String("2023-11-02 14:53:08+01:00".to_owned()), input);
        }

        {
            let timestamp: i64 = 1698933188760;
            let mut input = Value::from(timestamp);
            dateandtime("", &mut input, &Tz::America__New_York).unwrap();
            assert_eq!(Value::String("2023-11-02 09:53:08-04:00".to_owned()), input);
        }
    }

    #[test]
    fn datetime_modifier_should_convert_unix_timestamp_nanoseconds_to_datetime_string() {
        {
            let timestamp: i64 = 1698938840816144404;
            let mut input = Value::from(timestamp);
            dateandtime("", &mut input, &Tz::Europe__London).unwrap();
            assert_eq!(Value::String("2023-11-02 15:27:20+00:00".to_owned()), input);
        }

        {
            let timestamp: i64 = 1698938840816144404;
            let mut input = Value::from(timestamp);
            dateandtime("", &mut input, &Tz::Europe__Rome).unwrap();
            assert_eq!(Value::String("2023-11-02 16:27:20+01:00".to_owned()), input);
        }

        {
            let timestamp: i64 = 1698938840816144404;
            let mut input = Value::from(timestamp);
            dateandtime("", &mut input, &Tz::America__New_York).unwrap();
            assert_eq!(Value::String("2023-11-02 11:27:20-04:00".to_owned()), input);
        }
    }

    #[test]
    fn trim_modifier_should_fail_if_value_not_a_number() {
        {
            let mut input = Value::String("test".to_owned());
            assert!(dateandtime("", &mut input, &Tz::Europe__Rome).is_err());
        }

        {
            let mut input = Value::Array(vec![]);
            assert!(dateandtime("", &mut input, &Tz::Europe__Rome).is_err());
        }

        {
            let mut input = Value::Object(Map::new());
            assert!(dateandtime("", &mut input, &Tz::Europe__Rome).is_err());
        }

        {
            let mut input = Value::Bool(true);
            assert!(dateandtime("", &mut input, &Tz::Europe__Rome).is_err());
        }
    }
}
