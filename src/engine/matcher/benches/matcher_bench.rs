#[macro_use]
extern crate criterion;
extern crate serde;
extern crate serde_json;
extern crate tornado_common_api;
extern crate tornado_engine_matcher;

pub mod matcher;
pub mod utils;

use criterion::Criterion;
use crate::matcher::*;

criterion_group!(benches, one_simple_rule::bench, trap::bench);

criterion_main!(benches);
