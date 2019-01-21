use criterion::Criterion;
use std::fs;
use tornado_common_api::Event;

pub fn bench_jmespath_variable(c: &mut Criterion) {
    let filename = "./test_resources/event_nested_01.json";
    let event_json =
        fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

    c.bench_function("Parse nested json - jmespath", move |b| {
        b.iter(|| {
            assert!(jmespath::Variable::from_json(&event_json).is_ok());
        })
    });

    let filename = "./test_resources/event_01.json";
    let event_json =
        fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

    c.bench_function("Parse big json - jmespath", move |b| {
        b.iter(|| {
            assert!(jmespath::Variable::from_json(&event_json).is_ok());
        })
    });
}

pub fn bench_event(c: &mut Criterion) {
    let filename = "./test_resources/event_nested_01.json";
    let event_json =
        fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

    c.bench_function("Parse nested json - accessor", move |b| {
        b.iter(|| {
            assert!(serde_json::from_str::<Event>(&event_json).is_ok());
        })
    });

    let filename = "./test_resources/event_01.json";
    let event_json =
        fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

    c.bench_function("Parse big json - accessor", move |b| {
        b.iter(|| {
            assert!(serde_json::from_str::<Event>(&event_json).is_ok());
        })
    });
}
