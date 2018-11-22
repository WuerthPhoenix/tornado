use std::collections::HashMap;
use tornado_common_api::{Value, Payload};

// Temporal structure to be replaced with tornado_common_api::Value.
// Todo: to be removed when arrays are added to tornado_common_api::Value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum JsonValue {
    Text(String),
    Array(Vec<String>),
    Map(HashMap<String, String>),
}

impl Into<Value> for JsonValue {
    fn into(self) -> Value {
        match self {
            JsonValue::Text(text) => Value::Text(text),
            JsonValue::Array(values) => Value::Text(values.join(",")),
            JsonValue::Map(map) => {
                let payload: Payload = map.into_iter().map(|(key, json_value)| {
                    (key, Value::Text(json_value))
                }).collect();
                Value::Map(payload)
            },
        }
    }
}

impl<'o> Into<Option<&'o str>> for &'o JsonValue {
    fn into(self) -> Option<&'o str> {
        match self {
            JsonValue::Text(text) => Some(text),
            _ => None,
        }
    }
}

impl PartialEq<str> for JsonValue {
    fn eq(&self, other: &str) -> bool {
        let option_text: Option<&str> = self.into();
        match option_text {
            Some(text) => text == other,
            None => false,
        }
    }
}
// To make comparison bidirectional
impl PartialEq<JsonValue> for str {
    fn eq(&self, other: &JsonValue) -> bool {
        other == self
    }
}

pub type Json = HashMap<String, JsonValue>;

pub fn to_payload(json: Json) -> Payload {
    json.into_iter().map(|(key, json_value)| {
        (key, json_value.into())
    }).collect()

}

#[cfg(test)]
mod test {

    use super::*;
    use std::fs;

    #[test]
    fn should_parse_rsyslog_input_into_jsonvalue() {
        // Arrange
        let filename = "./test_resources/rsyslog_01_input.json";
        let event_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

        // Act
        let json = serde_json::from_str::<Json>(&event_json).unwrap();

        // Assert
        assert!(!json.is_empty());
        assert_eq!("2018-11-01T23:59:59+01:00", json.get("@timestamp").unwrap());
    }

    #[test]
    fn should_convert_array_into_comma_separated() {
        // Arrange
        let array = JsonValue::Array(vec![
            "first".to_owned(),
            "second".to_owned(),
            "third".to_owned()
        ]);

        // Act
        let value: Value = array.into();

        // Assert
        assert_eq!(Value::Text("first,second,third".to_owned()), value);
    }

}