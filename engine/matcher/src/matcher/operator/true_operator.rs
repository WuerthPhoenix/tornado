use crate::matcher::operator::Operator;
use crate::model::InternalEvent;
use std::collections::HashMap;
use tornado_common_api::Value;

const OPERATOR_NAME: &str = "true";

/// A matching matcher.operator that always evaluates to true
#[derive(Debug)]
pub struct True {}

impl Operator for True {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(
        &self,
        _event: &InternalEvent,
        _extracted_vars: Option<&HashMap<String, HashMap<String, Value>>>,
    ) -> bool {
        true
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use tornado_common_api::*;

    #[test]
    fn should_return_the_operator_name() {
        let operator = True {};
        assert_eq!(OPERATOR_NAME, operator.name());
    }

    #[test]
    fn should_return_true() {
        // Arrange
        let operator = True {};
        let event = Event::new("test_type");

        // Act
        let result = operator.evaluate(&InternalEvent::new(event), None);

        // Assert
        assert!(result);
    }
}
