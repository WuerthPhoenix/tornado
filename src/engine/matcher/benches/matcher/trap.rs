use criterion::Criterion;
use tornado_engine_matcher::matcher::Matcher;

use utils;

pub fn bench(c: &mut Criterion) {

    // Create rule
    let rule = utils::read_rule_from_file("./benches_resources/trap/rules/motion_sensor_4.json");

    // Create event
    let event= utils::read_event_from_file("./benches_resources/trap/events/event_01.json");

    // Create Matcher
    let matcher = Matcher::new(&vec![rule]).unwrap();

    //println!("result is : {:#?}", matcher.process(event.clone()));

    c.bench_function("Trap - Sensor 4", move |b| b.iter(||
        matcher.process(event.clone())
    ));

}
