use criterion::Criterion;
use tornado_common_api::Event;
use tornado_engine_matcher::matcher::Matcher;

use crate::utils;
use tornado_engine_matcher::config::MatcherConfig;

// Use case for an event with a big payload and a rule with two extracted variables that fully matches the event.
// This benchmark represents the "worst case" situation in which each event goes through the entire process.
pub fn bench(c: &mut Criterion) {
    // Create rule
    let rule = utils::read_rule_from_file("./benches_resources/full_match/rules/rule_01.json");

    // Create event
    let event = utils::read_event_from_file("./benches_resources/full_match/events/event_01.json");

    // Create Matcher
    let matcher =
        Matcher::build(&MatcherConfig::Ruleset { rules: vec![rule], name: "name".to_owned() })
            .unwrap();

    // println!("result is : {:?}", matcher.process(event.clone()));
    c.bench_function("Full match", move |b| b.iter(|| execute_test(&matcher, event.clone())));
}

fn execute_test(matcher: &Matcher, event: Event) {
    matcher.process(event);
}
