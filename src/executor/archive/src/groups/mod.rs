use regex::Regex;

const PATH_REGEX: &str = r"\$\{[^\}]+\}";

pub struct VarExtractor {
    regex: Regex,
}

impl Default for VarExtractor {
    fn default() -> Self {
        VarExtractor {
            regex: Regex::new(PATH_REGEX)
                .expect("VarExtractor regex should be valid"),
        }
    }
}

impl VarExtractor {
    pub fn new() -> VarExtractor {
        Default::default()
    }

    pub fn extract_vars(&self, source: &str) -> Vec<String> {
        let mut result = vec![];
        for capture in self.regex.captures_iter(source) {
            if let Some(value) = capture.get(0) {
                let group = value.as_str();
                result.push(group[2..group.len()-1].to_owned())
            }
        };
        result
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn should_extract_vars() {

        let extractor = VarExtractor::new();
        let empty : Vec<String> = vec![];

        assert_eq!(empty, extractor.extract_vars(""));
        assert_eq!(vec!["one", "two"], extractor.extract_vars("/dir/${one}/${two}"));
        assert_eq!(vec!["one_tw.o"], extractor.extract_vars("/dir/${one_tw.o}"));
    }

}

