use std::fs;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use tornado_common_api::Action;
use tornado_executor_common::StatelessExecutor;
use tornado_executor_foreach;
use tornado_executor_foreach::ForEachExecutor;
use tornado_network_simple::SimpleEventBus;

#[tokio::test]
async fn should_convert_value_to_action() {
    // Arrange
    let action_filename = "./test_resources/elasticsearch_usecase_01/action.json";
    let action_json = fs::read_to_string(action_filename)
        .expect(&format!("Unable to open the file [{}]", action_filename));
    let action: Action = serde_json::from_str(&action_json).unwrap();

    let expected_filename = "./test_resources/elasticsearch_usecase_01/expected.json";
    let expected_json = fs::read_to_string(expected_filename)
        .expect(&format!("Unable to open the file [{}]", expected_filename));
    let expected: Vec<Action> = serde_json::from_str(&expected_json).unwrap();

    let execution_results = Arc::new(RwLock::new(vec![]));

    let mut bus = SimpleEventBus::new();
    {
        let execution_results = execution_results.clone();
        bus.subscribe_to_action(
            "elasticsearch",
            Box::new(move |action| {
                let mut lock = execution_results.write().unwrap();
                lock.push(action);
            }),
        );
    };
    let executor = ForEachExecutor::new(Arc::new(bus));

    // Act
    executor.execute(action.into()).await.unwrap();

    // Assert
    let lock = execution_results.read().unwrap();
    assert_eq!(7, lock.len());
    assert_eq!(&expected, lock.deref());
}
