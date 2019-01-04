#[cfg(test)]
mod test {

    use std::fs;

    #[test]
    fn should_return_event_type() {

        // Arrange
        let filename = "./test_resources/event_nested_01.json";
        let event_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

        // Act
        let expr = jmespath::compile("type").unwrap();
        let data = jmespath::Variable::from_json(&event_json).unwrap();

        let result = expr.search(data).unwrap();

        // Assert
        assert_eq!("email", result.as_string().unwrap());
    }

    #[test]
    fn should_return_the_hostgroup() {

        // Arrange
        let filename = "./test_resources/event_nested_01.json";
        let event_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

        // Act
        let expr = jmespath::compile("payload.hostgroups[0]").unwrap();
        let data = jmespath::Variable::from_json(&event_json).unwrap();

        let result = expr.search(data).unwrap();

        // Assert
        assert_eq!("linux0", result.as_string().unwrap());
    }

    #[test]
    fn should_return_from_nested_map() {

        // Arrange
        let filename = "./test_resources/event_nested_01.json";
        let event_json =
            fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

        // Act
        let expr = jmespath::compile(r#"payload.first_level."second_level_text""#).unwrap();
        let data = jmespath::Variable::from_json(&event_json).unwrap();

        let result = expr.search(data).unwrap();

        // Assert
        assert_eq!("some text", result.as_string().unwrap());
    }

}
