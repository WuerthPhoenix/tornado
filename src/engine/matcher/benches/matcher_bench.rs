pub mod matcher;
pub mod utils;

use crate::matcher::*;
use criterion::*;

criterion_group!(benches, one_simple_rule::bench, no_match::bench, full_match::bench);

criterion_main!(benches);
