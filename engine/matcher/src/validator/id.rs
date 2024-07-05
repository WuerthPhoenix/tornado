use crate::error::MatcherError;
use crate::validator::NodePath;
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
    pub fn validate_rule_name(&self, parent: &NodePath, name: &str) -> Result<(), MatcherError> {
        let error_message = format!(
            "Rule name [{}] in ruleset [{}] is not valid. It should respect the pattern {}",
            name, parent, ID_REGEX_PATTERN
        );
        self.validate(name, error_message)
    }

    /// Validates a filter name.
    pub fn validate_filter_name(&self, parent: &NodePath, name: &str) -> Result<(), MatcherError> {
        let error_message = format!(
            "Filter name [{}] in path [{}] is not valid. It should respect the pattern {}",
            name, parent, ID_REGEX_PATTERN
        );
        self.validate(name, error_message)
    }

    /// Validates a iterator name.
    pub fn validate_iterator_name(
        &self,
        parent: &NodePath,
        name: &str,
    ) -> Result<(), MatcherError> {
        let error_message = format!(
            "Iterator name [{}] in path [{}] is not valid. It should respect the pattern {}",
            name, parent, ID_REGEX_PATTERN
        );
        self.validate(name, error_message)
    }

    /// Validates a ruleset name.
    pub fn validate_ruleset_name(&self, parent: &NodePath, name: &str) -> Result<(), MatcherError> {
        let error_message = format!(
            "Ruleset name [{}] in path [{}] is not valid. It should respect the pattern {}",
            name, parent, ID_REGEX_PATTERN
        );
        self.validate(name, error_message)
    }

    /// Validates an extracted variable name.
    pub fn validate_extracted_var_name(
        &self,
        parent: &NodePath,
        var_name: &str,
    ) -> Result<(), MatcherError> {
        let error_message = format!(
            "Variable name [{}] for rule [{}] is not valid. It should respect the pattern {}",
            var_name, parent, ID_REGEX_PATTERN
        );
        self.validate(var_name, error_message)
    }

    /// Validates an action ID.
    pub fn validate_action_id(
        &self,
        parent: &NodePath,
        action_id: &str,
    ) -> Result<(), MatcherError> {
        let error_message = format!(
            "Action id [{}] for rule [{}] is not valid. It should respect the pattern {}",
            action_id, parent, ID_REGEX_PATTERN
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

        assert!(id.validate_rule_name(&NodePath::Root, "hello").is_ok());
        assert!(id.validate_rule_name(&NodePath::Root, "helloWorld").is_ok());
        assert!(id.validate_rule_name(&NodePath::Root, "Hello_WORLD").is_ok());
        assert!(id.validate_rule_name(&NodePath::Root, "_").is_ok());
        assert!(id.validate_rule_name(&NodePath::Root, "1").is_ok());
        assert!(id.validate_rule_name(&NodePath::Root, "hello_10_world").is_ok());
        assert!(id.validate_rule_name(&NodePath::Root, "__0__C__").is_ok());

        assert!(id.validate_rule_name(&NodePath::Root, "").is_err());
        assert!(id.validate_rule_name(&NodePath::Root, " ").is_err());
        assert!(id.validate_rule_name(&NodePath::Root, "!").is_err());
        assert!(id.validate_rule_name(&NodePath::Root, "hello world").is_err());
        assert!(id.validate_rule_name(&NodePath::Root, "hello!").is_err());
        assert!(id.validate_rule_name(&NodePath::Root, "hello!?^ì").is_err());
    }

    #[test]
    fn should_check_if_valid_extracted_var_name() {
        let id = IdValidator::new();

        assert!(id.validate_extracted_var_name(&NodePath::Root, "hello").is_ok());
        assert!(id.validate_extracted_var_name(&NodePath::Root, "helloWorld").is_ok());
        assert!(id.validate_extracted_var_name(&NodePath::Root, "Hello_WORLD").is_ok());

        assert!(id.validate_extracted_var_name(&NodePath::Root, "").is_err());
        assert!(id.validate_extracted_var_name(&NodePath::Root, " ").is_err());
        assert!(id.validate_extracted_var_name(&NodePath::Root, "!").is_err());
    }

    #[test]
    fn should_check_if_valid_action_id() {
        let id = IdValidator::new();

        assert!(id.validate_action_id(&NodePath::Root, "hello").is_ok());
        assert!(id.validate_action_id(&NodePath::Root, "helloWorld").is_ok());
        assert!(id.validate_action_id(&NodePath::Root, "Hello_WORLD").is_ok());

        assert!(id.validate_action_id(&NodePath::Root, "").is_err());
        assert!(id.validate_action_id(&NodePath::Root, " ").is_err());
        assert!(id.validate_action_id(&NodePath::Root, "!").is_err());
    }
}
