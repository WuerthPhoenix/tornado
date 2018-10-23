use error::MatcherError;
use regex::Regex as RustRegex;

const ID_REGEX_PATTERN: &str = "^[a-zA-Z0-9_]+$";

pub trait IdValidator {
    fn validate_rule_name(&self, rule_name: &str) -> Result<(), MatcherError>;
    fn validate_extracted_var_name(&self, id: &str, rule_name: &str) -> Result<(), MatcherError>;
    fn validate_payload_key(&self, id: &str, rule_name: &str) -> Result<(), MatcherError>;
    fn validate_action_id(&self, id: &str, rule_name: &str) -> Result<(), MatcherError>;
}

impl IdValidator {
    pub fn new() -> impl IdValidator {
        RegexIdValidator {
            regex: RustRegex::new(ID_REGEX_PATTERN).unwrap(),
        }
    }
}

struct RegexIdValidator {
    regex: RustRegex,
}

impl RegexIdValidator {
    fn validate(&self, id: &str, error_message: String) -> Result<(), MatcherError> {
        if !self.regex.is_match(id) {
            return Err(MatcherError::NotValidIdOrNameError {
                message: error_message,
            });
        }
        Ok(())
    }
}

impl IdValidator for RegexIdValidator {
    fn validate_rule_name(&self, rule_name: &str) -> Result<(), MatcherError> {
        let error_message = format!(
            "Rule name [{}] is not valid. It should respect the pattern {}",
            rule_name, ID_REGEX_PATTERN
        );
        self.validate(rule_name, error_message)
    }

    fn validate_extracted_var_name(
        &self,
        var_name: &str,
        rule_name: &str,
    ) -> Result<(), MatcherError> {
        let error_message = format!(
            "Variable name [{}] for rule [{}] is not valid. It should respect the pattern {}",
            var_name, rule_name, ID_REGEX_PATTERN
        );
        self.validate(var_name, error_message)
    }

    fn validate_payload_key(&self, payload_key: &str, rule_name: &str) -> Result<(), MatcherError> {
        let error_message = format!(
            "Payload key [{}] for rule [{}] is not valid. It should respect the pattern {}",
            payload_key, rule_name, ID_REGEX_PATTERN
        );
        self.validate(payload_key, error_message)
    }

    fn validate_action_id(&self, action_id: &str, rule_name: &str) -> Result<(), MatcherError> {
        let error_message = format!(
            "Action id [{}] for rule [{}] is not valid. It should respect the pattern {}",
            action_id, rule_name, ID_REGEX_PATTERN
        );
        self.validate(action_id, error_message)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_check_if_valid_rule_name() {
        let id = IdValidator::new();

        assert!(id.validate_rule_name("hello").is_ok());
        assert!(id.validate_rule_name("helloWorld").is_ok());
        assert!(id.validate_rule_name("Hello_WORLD").is_ok());
        assert!(id.validate_rule_name("_").is_ok());
        assert!(id.validate_rule_name("1").is_ok());
        assert!(id.validate_rule_name("hello_10_world").is_ok());
        assert!(id.validate_rule_name("__0__C__").is_ok());

        assert!(id.validate_rule_name("").is_err());
        assert!(id.validate_rule_name(" ").is_err());
        assert!(id.validate_rule_name("!").is_err());
        assert!(id.validate_rule_name("hello world").is_err());
        assert!(id.validate_rule_name("hello!").is_err());
        assert!(id.validate_rule_name("hello!?^ì").is_err());
    }

    #[test]
    fn should_check_if_valid_extracted_var_name() {
        let id = IdValidator::new();

        assert!(id.validate_extracted_var_name("hello", "rule").is_ok());
        assert!(id.validate_extracted_var_name("helloWorld", "rule").is_ok());
        assert!(
            id.validate_extracted_var_name("Hello_WORLD", "rule")
                .is_ok()
        );

        assert!(id.validate_extracted_var_name("", "rule").is_err());
        assert!(id.validate_extracted_var_name(" ", "rule").is_err());
        assert!(id.validate_extracted_var_name("!", "rule").is_err());
    }

    #[test]
    fn should_check_if_valid_payload_key() {
        let id = IdValidator::new();

        assert!(id.validate_payload_key("hello", "rule").is_ok());
        assert!(id.validate_payload_key("helloWorld", "rule").is_ok());
        assert!(id.validate_payload_key("Hello_WORLD", "rule").is_ok());

        assert!(id.validate_payload_key("", "rule").is_err());
        assert!(id.validate_payload_key(" ", "rule").is_err());
        assert!(id.validate_payload_key("!", "rule").is_err());
    }

    #[test]
    fn should_check_if_valid_action_id() {
        let id = IdValidator::new();

        assert!(id.validate_action_id("hello", "rule").is_ok());
        assert!(id.validate_action_id("helloWorld", "rule").is_ok());
        assert!(id.validate_action_id("Hello_WORLD", "rule").is_ok());

        assert!(id.validate_action_id("", "rule").is_err());
        assert!(id.validate_action_id(" ", "rule").is_err());
        assert!(id.validate_action_id("!", "rule").is_err());
    }

}
