use serde_json::{Map, Value};
use std::collections::HashMap;

pub type Payload = Map<String, Value>;

pub trait ValueGet {
    fn get_from_map(&self, key: &str) -> Option<&Value>;
    fn get_from_array(&self, index: usize) -> Option<&Value>;
}

impl<'o> ValueGet for HashMap<&'o str, &'o Value> {
    fn get_from_map(&self, key: &str) -> Option<&Value> {
        self.get(key).copied()
    }
    fn get_from_array(&self, _index: usize) -> Option<&Value> {
        None
    }
}

impl ValueGet for Map<String, Value> {
    fn get_from_map(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }
    fn get_from_array(&self, _index: usize) -> Option<&Value> {
        None
    }
}

impl ValueGet for HashMap<String, Value> {
    fn get_from_map(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }
    fn get_from_array(&self, _index: usize) -> Option<&Value> {
        None
    }
}

impl ValueGet for Value {
    fn get_from_map(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(payload) => payload.get(key),
            _ => None,
        }
    }
    fn get_from_array(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Array(array) => array.get(index),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ValueGet;
    use serde_json::{Map, Value};

    #[test]
    fn value_text_should_return_no_child() {
        // Arrange
        let value = Value::String("".to_owned());

        // Act
        let result = value.get_from_map("");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn value_map_should_return_child_if_exists() {
        // Arrange
        let mut children = Map::new();
        children.insert("first".to_owned(), Value::String("first_value".to_owned()));
        children.insert("second".to_owned(), Value::String("second_value".to_owned()));

        let value = Value::Object(children);

        // Act
        let result = value.get_from_map("second");

        // Assert
        assert!(result.is_some());
        assert_eq!("second_value", result.unwrap());
    }

    #[test]
    fn value_map_should_return_no_child_if_absent() {
        // Arrange
        let mut children = Map::new();
        children.insert("first".to_owned(), Value::String("first_value".to_owned()));
        children.insert("second".to_owned(), Value::String("second_value".to_owned()));

        let value = Value::Object(children);

        // Act
        let result = value.get_from_map("third");

        // Assert
        assert!(result.is_none());
    }
}
