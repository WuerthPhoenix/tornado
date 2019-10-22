use cpuprofiler::PROFILER;
use criterion::Criterion;
use tornado_common_api::{Event, Payload, Value};

use tornado_engine_matcher::interpolator::StringInterpolator;
use tornado_engine_matcher::model::InternalEvent;

pub fn bench(c: &mut Criterion) {
    let mut payload = Payload::new();
    payload.insert("body".to_owned(), Value::Text("body_value".to_owned()));
    payload.insert("subject".to_owned(), Value::Text("subject_value".to_owned()));

    let event = InternalEvent::new(Event {
        event_type: "event_type_value".to_owned(),
        created_ms: 1554130814854,
        payload,
    });

    let template = "type: ${event.type} - body: ${event.payload.body}";

    let interpolator = StringInterpolator::build(template, "rule", &Default::default()).unwrap();

    PROFILER.lock().unwrap().start("./target/interpolator.profile").unwrap();
    c.bench_function("String interpolator", move |b| {
        b.iter(|| execute_test(&interpolator, &event))
    });
    PROFILER.lock().unwrap().stop().unwrap();
}

fn execute_test(interpolator: &StringInterpolator, event: &InternalEvent) {
    assert!(interpolator.render(event, None).is_ok());
}
