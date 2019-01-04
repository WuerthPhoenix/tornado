pub mod tests;
use criterion::*;

use crate::tests::*;

criterion_group!(benches,
    extract_event_type::bench_jmespath,
    extract_event_type::bench_accessor,
    extract_from_array::bench_jmespath,
    extract_from_array::bench_accessor,
    extract_from_map::bench_jmespath,
    extract_from_map::bench_accessor,
    );

criterion_main!(benches);
