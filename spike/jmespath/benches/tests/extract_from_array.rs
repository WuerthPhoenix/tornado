use criterion::Criterion;
use std::fs;
use tornado_common_api::{Event};
use tornado_engine_matcher::accessor::AccessorBuilder;
use tornado_engine_matcher::model::InternalEvent;

pub fn bench_jmespath(c: &mut Criterion) {
    let filename = "./test_resources/event_nested_01.json";
    let event_json =
        fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

    let expr = jmespath::compile("payload.hostgroups[0]").unwrap();
    let data = jmespath::Variable::from_json(&event_json).unwrap();

    c.bench_function("Extract from array - jmespath", move |b| {
        b.iter(|| {
            let result = expr.search(data.clone()).unwrap();
            // Assert
            assert_eq!("linux0", result.as_string().unwrap());
        })
    });
}

pub fn bench_accessor(c: &mut Criterion) {
    let filename = "./test_resources/event_nested_01.json";
    let event_json =
        fs::read_to_string(filename).expect(&format!("Unable to open the file [{}]", filename));

    let builder = AccessorBuilder::new();
    let value = "${event.payload.hostgroups[0]}".to_owned();
    let accessor = builder.build("", &value).unwrap();
    let event = serde_json::from_str::<Event>(&event_json).unwrap();
    let internal_event = InternalEvent::new(event);

    c.bench_function("Extract from array - accessor", move |b| {
        b.iter(|| {
            let internal_event_clone = internal_event.clone();
            let result = accessor.get(&internal_event_clone, None);
            // Assert
            assert_eq!("linux0", result.unwrap().as_ref().get_text().unwrap());
        })
    });
}
