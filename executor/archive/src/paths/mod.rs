use log::*;
use regex::Regex;
use tornado_common_api::Payload;
use tornado_executor_common::ExecutorError;

const PATH_REGEX: &str = r"\$\{[^\}]+\}";

pub struct PathMatcherBuilder {
    regex: Regex,
}

impl Default for PathMatcherBuilder {
    fn default() -> Self {
        PathMatcherBuilder {
            regex: Regex::new(PATH_REGEX).expect("VarExtractor regex should be valid"),
        }
    }
}

impl PathMatcherBuilder {
    pub fn new() -> PathMatcherBuilder {
        Default::default()
    }

    pub fn build<S: Into<String>>(&self, source: S) -> PathMatcher {
        let path = source.into();
        let mut variables = vec![];
        for capture in self.regex.captures_iter(&path) {
            if let Some(value) = capture.get(0) {
                let group = value.as_str();
                let param = Parameter {
                    simple: group[2..group.len() - 1].to_owned(),
                    full: group.to_owned(),
                };
                variables.push(param)
            }
        }
        PathMatcher { path, parameters: variables }
    }
}

pub struct PathMatcher {
    path: String,
    parameters: Vec<Parameter>,
}

#[derive(Debug, PartialEq)]
pub struct Parameter {
    pub simple: String,
    pub full: String,
}

impl PathMatcher {
    pub fn build_path(&self, payload: &Payload) -> Result<String, ExecutorError> {
        let mut path = self.path.clone();
        for param in self.parameters.iter() {
            let var_value = payload
                .get(&param.simple)
                .and_then(tornado_common_api::ValueExt::get_text)
                .ok_or_else(|| {
                    let message = format!(
                        "Cannot resolve path parameter [{}] for path [{}]",
                        &param.simple, self.path
                    );
                    warn!("{}", &message);
                    ExecutorError::ActionExecutionError {
                        can_retry: false,
                        message,
                        code: None,
                        data: Default::default(),
                    }
                })?;
            path = path.replace(&param.full, var_value);
        }
        Ok(path)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use tornado_common_api::Value;

    #[test]
    fn should_extract_parameters() {
        // Arrange
        let builder = PathMatcherBuilder::new();
        let empty: Vec<Parameter> = vec![];

        // Assert
        assert_eq!(empty, builder.build("").parameters);
        assert_eq!(
            vec![
                Parameter { simple: "one".to_owned(), full: "${one}".to_owned() },
                Parameter { simple: "two".to_owned(), full: "${two}".to_owned() }
            ],
            builder.build("/dir/${one}/${two}").parameters
        );
        assert_eq!(
            vec![Parameter { simple: "one_tw.o".to_owned(), full: "${one_tw.o}".to_owned() }],
            builder.build("/dir/${one_tw.o}").parameters
        );
    }

    #[test]
    fn should_return_expected_path() {
        // Arrange
        let builder = PathMatcherBuilder::new();
        let path_matcher = builder.build("/dir/${one}/${two}");

        let mut payload = Payload::new();
        payload.insert("one".to_owned(), Value::Text("one_value".to_owned()));
        payload.insert("two".to_owned(), Value::Text("two_value".to_owned()));

        // Act
        let result = path_matcher.build_path(&payload).unwrap();

        // Assert
        assert_eq!("/dir/one_value/two_value", result);
    }

    #[test]
    fn should_return_error_if_missing_variables() {
        // Arrange
        let builder = PathMatcherBuilder::new();
        let path_matcher = builder.build("/dir/${one}/${two}");

        let payload = Payload::new();

        // Act
        let result = path_matcher.build_path(&payload);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_return_error_if_at_least_one_var_is_missing() {
        // Arrange
        let builder = PathMatcherBuilder::new();
        let path_matcher = builder.build("/dir/${one}/${two}/${three}");

        let mut payload = Payload::new();
        payload.insert("one".to_owned(), Value::Text("one_value".to_owned()));
        payload.insert("two".to_owned(), Value::Text("two_value".to_owned()));

        // Act
        let result = path_matcher.build_path(&payload);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_resolve_path_with_repeated_vars() {
        // Arrange
        let builder = PathMatcherBuilder::new();
        let path_matcher = builder.build("/dir/${one}/${two}/${one}");

        let mut payload = Payload::new();
        payload.insert("one".to_owned(), Value::Text("one_value".to_owned()));
        payload.insert("two".to_owned(), Value::Text("two_value".to_owned()));

        // Act
        let result = path_matcher.build_path(&payload).unwrap();

        // Assert
        assert_eq!("/dir/one_value/two_value/one_value", result);
    }

    #[test]
    fn should_always_return_path_if_it_has_no_parameters() {
        // Arrange
        let builder = PathMatcherBuilder::new();
        let path_matcher = builder.build("/dir/one/two");

        let payload = Payload::new();

        // Act
        let result = path_matcher.build_path(&payload).unwrap();

        // Assert
        assert_eq!("/dir/one/two", result);
    }
}
