use accessor::AccessorBuilder;
use error::MatcherError;
use rule;

/// Rule instance builder.
pub struct RuleBuilder {
    accessor: AccessorBuilder,
    delimiter: &'static str,
    array_start: &'static str,
    array_end: &'static str,
}

impl RuleBuilder {
    pub fn new() -> RuleBuilder {
        RuleBuilder {
            accessor: AccessorBuilder::new(),
            delimiter: ",",
            array_start: "[",
            array_end: "]",
        }
    }

    /// Parses a string representing a rule operator into an array or strings where each string
    /// is an argument of the operator.
    /// This operation is not recursive.
    ///
    /// # Example 1
    ///
    /// Parsing a simple array representation:
    ///
    /// ```rust
    /// extern crate tornado_engine_matcher;
    /// use tornado_engine_matcher::rule::parser::RuleBuilder;
    ///
    /// let builder = RuleBuilder::new();
    /// let rule = "[one,two,three]";
    ///
    /// let args = builder.parse(rule.to_owned()).unwrap();
    ///
    /// assert_eq!(vec![
    ///     "one".to_owned(),
    ///     "two".to_owned(),
    ///     "three".to_owned()
    /// ], args);
    /// ```
    ///
    /// # Example 2
    ///
    /// Parsing an array with nested children:
    ///
    /// ```rust
    /// extern crate tornado_engine_matcher;
    /// use tornado_engine_matcher::rule::parser::RuleBuilder;
    ///
    /// let builder = RuleBuilder::new();
    /// let rule = "[one,[in1, in2],two,three]";
    ///
    /// let args = builder.parse(rule.to_owned()).unwrap();
    ///
    /// assert_eq!(vec![
    ///     "one".to_owned(),
    ///     "[in1, in2]".to_owned(),
    ///     "two".to_owned(),
    ///     "three".to_owned()
    /// ], args);
    /// ```
    pub fn parse(&self, rule: String) -> Result<Vec<String>, MatcherError> {
        let input = self.check_array_start_end(&rule)?;

        let mut start = 0;
        let mut parenthesis_count = 0;
        let mut result: Vec<String> = vec![];

        for (i, c) in input.chars().enumerate() {
            let char_string = c.to_string();
            let char_str = char_string.as_str();

            if char_str == self.array_start {
                parenthesis_count += 1;
            } else if char_str == self.array_end {
                parenthesis_count -= 1;
            }

            if i == (input.len() - 1) {
                result.push((&input[start..]).to_owned());
            } else if (char_str == self.delimiter) && (parenthesis_count == 0) {
                result.push((&input[start..i]).to_owned());
                start = i + 1;
            }
        }

        Ok(result)
    }

    fn check_array_start_end<'o>(&self, rule: &'o String) -> Result<&'o str, MatcherError> {
        let input = rule.trim();

        if !input.starts_with(self.array_start) {
            return Err(MatcherError::ParseOperatorError {
                message: format!(
                    "Expected to start with [{}], found [{}]",
                    self.array_start, input
                ),
            });
        }

        if !input.ends_with(self.array_end) {
            return Err(MatcherError::ParseOperatorError {
                message: format!(
                    "Expected to end with [{}], found [{}]",
                    self.array_end, input
                ),
            });
        }

        let array_start_count = input.matches(self.array_start).count();
        let array_end_count = input.matches(self.array_end).count();

        if array_start_count != array_end_count {
            return Err(MatcherError::ParseOperatorError {
                message: format!("Wrong number of open/close array delimiters. Found [{}] occurrences of [{}] and [{}] occurrences of [{}]. The should be equals.",
                                 array_start_count, self.array_start, array_end_count, self.array_end)
            });
        }

        Ok(&input[1..input.len() - 1])
    }

    /// Returns a specific Rule instance based on operator defined on the first position of the args vector.
    ///
    /// # Example
    ///
    /// ```rust
    ///
    /// extern crate tornado_engine_matcher;
    ///
    /// use tornado_engine_matcher::rule::parser::RuleBuilder;
    ///
    ///       let args = vec![
    ///            "=".to_owned(), // "=" is the rule operator
    ///            "first_arg=".to_owned(),
    ///            "second_arg".to_owned()
    ///        ];
    ///
    /// let builder = RuleBuilder::new();
    /// let rule = builder.build(&args).unwrap(); // rule is an instance of EqualRule
    /// ```
    pub fn build(&self, args: &Vec<String>) -> Result<Box<rule::Rule>, MatcherError> {
        if args.is_empty() {
            return Err(MatcherError::MissingOperatorError {});
        }

        let operator = args[0].as_ref();
        let mut params = args.to_owned();
        params.remove(0);
        match operator {
            "=" => Ok(Box::new(rule::rules::equal::EqualRule::build(
                &params,
                &self.accessor,
            )?)),
            "and" => Ok(Box::new(rule::rules::and::AndRule::build(&params, &self)?)),
            "or" => Ok(Box::new(rule::rules::or::OrRule::build(&params, &self)?)),
            "regex" => Ok(Box::new(rule::rules::regex::RegexRule::build(
                &params,
                &self.accessor,
            )?)),
            _ => Err(MatcherError::UnknownOperatorError {
                operator: operator.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn build_should_return_error_if_missing_operator() {
        let args = vec![];

        let builder = RuleBuilder::new();
        assert!(builder.build(&args).is_err());
    }

    #[test]
    fn build_should_return_error_if_unknown_operator() {
        let args = vec!["unknown".to_owned()];

        let builder = RuleBuilder::new();
        assert!(builder.build(&args).is_err());
    }

    #[test]
    fn build_should_return_the_equal_rule() {
        let args = vec![
            "=".to_owned(),
            "first_arg=".to_owned(),
            "second_arg".to_owned(),
        ];

        let builder = RuleBuilder::new();
        let rule = builder.build(&args).unwrap();

        assert_eq!("equal", rule.name());
    }

    #[test]
    fn build_should_return_the_regex_rule() {
        let args = vec!["regex".to_owned(), "reg".to_owned(), "target".to_owned()];

        let builder = RuleBuilder::new();
        let rule = builder.build(&args).unwrap();

        assert_eq!("regex", rule.name());
    }

    #[test]
    fn build_should_return_the_and_rule() {
        let builder = RuleBuilder::new();

        let args = builder.parse(r#"[and,[=,1,1]]"#.to_owned()).unwrap();
        let rule = builder.build(&args).unwrap();

        assert_eq!("and", rule.name());
    }

    #[test]
    fn build_should_return_the_or_rule() {
        let builder = RuleBuilder::new();

        let args = builder.parse(r#"[or,[=,1,1]]"#.to_owned()).unwrap();
        let rule = builder.build(&args).unwrap();

        assert_eq!("or", rule.name());
    }

    #[test]
    fn parse_should_fail_if_string_does_not_start_with_square_braket() {
        let builder = RuleBuilder::new();
        let rule = "one,two,three]";
        assert!(builder.parse(rule.to_owned()).is_err());
    }

    #[test]
    fn parse_should_fail_if_string_does_not_end_with_square_braket() {
        let builder = RuleBuilder::new();
        let rule = "[one,two,three";
        assert!(builder.parse(rule.to_owned()).is_err());
    }

    #[test]
    fn parse_should_fail_if_not_closed_properly_arrays() {
        let builder = RuleBuilder::new();
        let rule = "[one,[in1, in2],[two,three]";
        assert!(builder.parse(rule.to_owned()).is_err());
    }

    #[test]
    fn should_parse_a_comma_separated_array() {
        let builder = RuleBuilder::new();
        let rule = "[one,two,three]";

        let args = builder.parse(rule.to_owned()).unwrap();

        assert_eq!(
            vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
            args
        );
    }

    #[test]
    fn should_parse_a_empty_array() {
        let builder = RuleBuilder::new();
        let rule = "[]";

        let args = builder.parse(rule.to_owned()).unwrap();

        assert_eq!(0, args.len());
    }

    #[test]
    fn should_parse_a_single_element_array() {
        let builder = RuleBuilder::new();
        let rule = "[hello]";

        let args = builder.parse(rule.to_owned()).unwrap();

        assert_eq!(vec!["hello".to_owned(),], args);
    }

    #[test]
    fn parse_should_detect_correctly_a_nested_array() {
        let builder = RuleBuilder::new();
        let rule = "[one,[in1, in2],two,three]";

        let args = builder.parse(rule.to_owned()).unwrap();

        assert_eq!(
            vec![
                "one".to_owned(),
                "[in1, in2]".to_owned(),
                "two".to_owned(),
                "three".to_owned()
            ],
            args
        );
    }

    #[test]
    fn parse_should_detect_recursively_nested_arrays() {
        let builder = RuleBuilder::new();
        let rule = "[one,[in1, in2],two,[three],[[four,[five,six]], seven]]";

        let args = builder.parse(rule.to_owned()).unwrap();

        assert_eq!(
            vec![
                "one".to_owned(),
                "[in1, in2]".to_owned(),
                "two".to_owned(),
                "[three]".to_owned(),
                "[[four,[five,six]], seven]".to_owned(),
            ],
            args
        );
    }

}
