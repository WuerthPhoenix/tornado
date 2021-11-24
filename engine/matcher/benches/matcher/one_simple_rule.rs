use criterion::Criterion;
use serde_json::Map;
use serde_json::json;
use std::collections::BTreeMap;
use tornado_common_api::{Event, Value};
use tornado_engine_matcher::config::rule::*;
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::matcher::Matcher;

pub fn bench(c: &mut Criterion) {
    // Create rule
    let rule = {
        let mut rule = new_rule(
            "rule_name",
            Operator::Equals {
                first: Value::String("${event.type}".to_owned()),
                second: Value::String("email".to_owned()),
            },
        );

        // Add constraint
        rule.constraint.with.insert(
            String::from("extracted_var"),
            Extractor::Regex(ExtractorRegex {
                from: String::from("${event.payload.body}"),
                regex: ExtractorRegexType::Regex {
                    regex: String::from(r"[0-9]+"),
                    group_match_idx: Some(0),
                    all_matches: None,
                },
                modifiers_post: vec![],
            }),
        );

        // Add action
        let mut action = Action { id: "log".to_owned(), payload: Map::new() };

        action
            .payload
            .insert("var".to_owned(), Value::String("${_variables.extracted_var}".to_owned()));
        rule.actions.push(action);
        rule
    };

    // Create Matcher
    let matcher =
        Matcher::build(&MatcherConfig::Ruleset { rules: vec![rule], name: "name".to_owned() })
            .unwrap();

    // Create event
    let event = {
        let mut event = Event::new("email".to_owned());
        event.payload.insert("body".to_owned(), Value::String("45 degrees".to_owned()));
        event
    };

    // println!("result is : {:?}", matcher.process(event.clone()));
    let event = json!(event);
    c.bench_function("One simple rule", move |b| b.iter(|| matcher.process(event.clone(), false)));
}

fn new_rule(name: &str, operator: Operator) -> Rule {
    let constraint = Constraint { where_operator: Some(operator), with: BTreeMap::new() };

    Rule {
        name: name.to_owned(),
        do_continue: true,
        active: true,
        actions: vec![],
        description: "".to_owned(),
        constraint,
    }
}
