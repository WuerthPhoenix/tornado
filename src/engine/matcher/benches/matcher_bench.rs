pub mod matcher;
pub mod utils;

use crate::matcher::*;
use criterion::*;

criterion_group!(benches,
    full_match::bench,
    interpolator::bench,
    no_match::bench,
    one_simple_rule::bench
    );

criterion_main!(benches);
