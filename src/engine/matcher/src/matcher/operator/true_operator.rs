use matcher::operator::Operator;
use model::ProcessedEvent;

const OPERATOR_NAME: &str = "true";

/// A matching matcher.operator that always evaluates to true.
#[derive(Debug)]
pub struct True {
}

impl Operator for True {
    fn name(&self) -> &str {
        OPERATOR_NAME
    }

    fn evaluate(&self, _event: &ProcessedEvent) -> bool {
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
        let result = operator.evaluate(&ProcessedEvent::new(event));

        // Assert
        assert!(result);
    }

}
