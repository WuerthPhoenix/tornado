use cpuprofiler::PROFILER;
use criterion::Criterion;
use tornado_common_api::Event;
use tornado_engine_matcher::matcher::Matcher;

use crate::utils;
use tornado_engine_matcher::config::MatcherConfig;

// Use case with a single rule and a single event that does not match the rule.
// This benchmark represents the "best case" situation from a performance point of view.
pub fn bench(c: &mut Criterion) {
    // Create rule
    let rule = utils::read_rule_from_file("./benches_resources/no_match/rules/rule_01.json");

    // Create event
    let event = utils::read_event_from_file("./benches_resources/no_match/events/event_01.json");

    // Create Matcher
    let matcher = Matcher::build(&MatcherConfig::Ruleset { rules: vec![rule], name: "name".to_owned() }).unwrap();

    // println!("result is : {:?}", matcher.process(event.clone()));
    PROFILER.lock().unwrap().start("./target/no_match.profile").unwrap();
    c.bench_function("No match", move |b| b.iter(|| execute_test(&matcher, event.clone())));
    PROFILER.lock().unwrap().stop().unwrap();
}

fn execute_test(matcher: &Matcher, event: Event) {
    matcher.process(event);
}
