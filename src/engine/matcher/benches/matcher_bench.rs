#[macro_use]
extern crate criterion;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tornado_common_api;
extern crate tornado_common_logger;
extern crate tornado_engine_matcher;

pub mod matcher;
pub mod utils;

use criterion::Criterion;
use matcher::*;

criterion_group!(benches,
    //one_simple_rule::bench,
    trap::bench);

criterion_main!(benches);