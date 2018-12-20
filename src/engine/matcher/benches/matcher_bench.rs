pub mod matcher;
pub mod utils;

use crate::matcher::*;
use criterion::*;

criterion_group!(benches, one_simple_rule::bench, trap::bench);

criterion_main!(benches);
