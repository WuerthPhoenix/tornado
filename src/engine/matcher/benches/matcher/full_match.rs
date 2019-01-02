use cpuprofiler::PROFILER;
use criterion::Criterion;
use tornado_common_api::Event;
use tornado_engine_matcher::matcher::Matcher;

use crate::utils;

// Use case for an event with a big payload and a rule with two extracted variables that fully matches the event.
// This bench represents the "worst case" situation in which each event goes through the entire process.
pub fn bench(c: &mut Criterion) {
    // Create rule
    let rule = utils::read_rule_from_file("./benches_resources/full_match/rules/rule_01.json");

    // Create event
    let event = utils::read_event_from_file("./benches_resources/full_match/events/event_01.json");

    // Create Matcher
    let matcher = Matcher::build(&vec![rule]).unwrap();

    // println!("result is : {:#?}", matcher.process(event.clone()));
    PROFILER.lock().unwrap().start("./target/full_match.profile").unwrap();
    c.bench_function("Full match", move |b| b.iter(|| execute_test(&matcher, event.clone())));
    PROFILER.lock().unwrap().stop().unwrap();
}

fn execute_test(matcher: &Matcher, event: Event) {
    matcher.process(event);
}
