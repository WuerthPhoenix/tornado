use crate::error::MatcherError;
use regex::Regex as RustRegex;

const ID_REGEX_PATTERN: &str = "^[a-zA-Z0-9_]+$";

/// A validator for name and ID
/// It checks that a string is composed only of alphabetical characters, numbers, and the '_' character.
pub struct IdValidator {
    regex: RustRegex,
}

impl Default for IdValidator {
    fn default() -> Self {
        IdValidator::new()
    }
}

impl IdValidator {
    pub fn new() -> IdValidator {
        IdValidator { regex: RustRegex::new(ID_REGEX_PATTERN).unwrap() }
    }

    /// Validates a generic ID or name.
    fn validate(&self, id: &str, error_message: String) -> Result<(), MatcherError> {
        if !self.regex.is_match(id) {
            return Err(MatcherError::NotValidIdOrNameError { message: error_message });
        }
        Ok(())
    }

    /// Validates a rule name.
    pub fn validate_rule_name(&self, rule_name: &str) -> Result<(), MatcherError> {
        let error_message = format!(
            "Rule name [{}] is not valid. It should respect the pattern {}",
            rule_name, ID_REGEX_PATTERN
        );
        self.validate(rule_name, error_message)
    }

    /// Validates a filter name.
    pub fn validate_filter_name(&self, filter_name: &str) -> Result<(), MatcherError> {
        let error_message = format!(
            "Filter name [{}] is not valid. It should respect the pattern {}",
            filter_name, ID_REGEX_PATTERN
        );
        self.validate(filter_name, error_message)
    }

    /// Validates a ruleset name.
    pub fn validate_ruleset_name(&self, filter_name: &str) -> Result<(), MatcherError> {
        let error_message = format!(
            "Ruleset name [{}] is not valid. It should respect the pattern {}",
            filter_name, ID_REGEX_PATTERN
        );
        self.validate(filter_name, error_message)
    }

    /// Validates an extracted variable name.
    pub fn validate_extracted_var_name(
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

    /// Validates an extracted variable name from an accessor.
    pub fn validate_extracted_var_from_accessor(
        &self,
        extracted_var: &str,
        full_accessor: &str,
        rule_name: &str,
    ) -> Result<(), MatcherError> {
        let error_message = format!(
            "Variable key [{}] from accessor [{}] for rule [{}] is not valid. It should respect the pattern {}",
            extracted_var, full_accessor, rule_name, ID_REGEX_PATTERN
        );
        self.validate(extracted_var, error_message)
    }

    /// Validates an action ID.
    pub fn validate_action_id(&self, action_id: &str, rule_name: &str) -> Result<(), MatcherError> {
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
        assert!(id.validate_extracted_var_name("Hello_WORLD", "rule").is_ok());

        assert!(id.validate_extracted_var_name("", "rule").is_err());
        assert!(id.validate_extracted_var_name(" ", "rule").is_err());
        assert!(id.validate_extracted_var_name("!", "rule").is_err());
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
