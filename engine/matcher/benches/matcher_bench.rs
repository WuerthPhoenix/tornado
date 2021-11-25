pub mod matcher;
pub mod spike;
pub mod utils;

use crate::matcher::*;
use criterion::*;

criterion_group!(
    benches,
    full_match::bench,
    no_match::bench,
    one_simple_rule::bench,
    spike::start_with::bench,
);

criterion_main!(benches);
