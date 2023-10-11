use crate::error::SmsParseError;
use chrono::NaiveDateTime;
use serde::de::Visitor;
use serde::{de, Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Formatter;

#[derive(Serialize, Deserialize, Debug)]
pub struct SmsEventPayload {
    #[serde(alias = "From")]
    sender: String,
    #[serde(alias = "Sent", deserialize_with = "deserialize_timestamp_from_datetime_string")]
    timestamp: i64,
    // This is not documented in the official documentation, but a test on the rdneteye showed that
    // the field is infact there.
    #[serde(alias = "Modem")]
    modem: String,
    hostname: String,
    text: String,
}

pub fn parse_sms(sms: &str) -> Result<SmsEventPayload, SmsParseError> {
    let Some((headers, text)) = sms.split_once("\n\n") else {
        return Err(SmsParseError::FormatError("The sms does not have the expected format, the text seems to be missing.".to_owned()))
    };

    let mut fields = HashMap::new();

    for (line_num, line) in headers.lines().enumerate() {
        if let Some((key, value)) = line.split_once(':') {
            fields.insert(key.trim(), value.trim());
        } else {
            return Err(SmsParseError::FormatError(format!(
                "Line {}: Missing colon, invalid header format!",
                line_num + 1
            )));
        }
    }

    fields.insert("text", text.trim());

    let hostname = gethostname::gethostname();
    let Some(hostname) = hostname.to_str() else {
        return Err(SmsParseError::HostnameFormatError);
    };

    fields.insert("hostname", hostname);

    match serde_json::to_value(fields).and_then(serde_json::from_value) {
        Ok(value) => Ok(value),
        Err(error) => Err(SmsParseError::ContentError(error)),
    }
}

pub fn deserialize_timestamp_from_datetime_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct DateTimeVisitor;
    impl Visitor<'_> for DateTimeVisitor {
        type Value = i64;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("Sent datetime not formatted as expected: dd-mm-yy HH:MM:SS")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match NaiveDateTime::parse_from_str(v, "%y-%m-%d %H:%M:%S") {
                Ok(value) => Ok(value.timestamp()),
                Err(err) => Err(serde::de::Error::custom(err)),
            }
        }
    }

    deserializer.deserialize_str(DateTimeVisitor)
}

#[cfg(test)]
mod test {
    use crate::error::SmsParseError;
    use crate::sms::parse_sms;

    #[test]
    fn should_parse_sms_correctly() {
        let sms = include_str!("../test_sms/example_sms_minimal");

        let sms_event_payload = parse_sms(sms).unwrap();

        assert_eq!(sms_event_payload.sender, "393333333333");
        assert_eq!(sms_event_payload.timestamp, 1696853719);
        assert_eq!(sms_event_payload.modem, "GSM1");
        assert_eq!(sms_event_payload.text, "Test 3");
    }

    #[test]
    fn should_parse_sms_with_extra_fields_correctly() {
        let sms = include_str!("../test_sms/example_sms");

        let sms_event_payload = parse_sms(sms).unwrap();

        assert_eq!(sms_event_payload.sender, "393333333333");
        assert_eq!(sms_event_payload.timestamp, 1696853719);
        assert_eq!(sms_event_payload.modem, "GSM1");
        assert_eq!(sms_event_payload.text, "Test 3");
    }

    #[test]
    fn should_fail_parsing_sms_on_missing_header() {
        let sms = include_str!("../test_sms/example_sms_missing_header");

        let sms_event_payload = parse_sms(sms);
        assert!(matches!(sms_event_payload, Err(SmsParseError::ContentError(_))))
    }

    #[test]
    fn should_fail_parsing_sms_on_broken_header() {
        let sms = include_str!("../test_sms/example_sms_broken_header");

        let sms_event_payload = parse_sms(sms);

        assert!(matches!(sms_event_payload, Err(SmsParseError::FormatError(_))))
    }

    #[test]
    fn should_fail_parsing_sms_on_missing_text() {
        let sms = include_str!("../test_sms/example_sms_missing_text");

        let sms_event_payload = parse_sms(sms);
        assert!(matches!(sms_event_payload, Err(SmsParseError::FormatError(_))))
    }
}
