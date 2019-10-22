pub mod tests;
use criterion::*;

use crate::tests::*;

criterion_group!(
    benches,
    from_json::bench_jmespath_variable,
    from_json::bench_event,
    extract_event_type::bench_jmespath_1,
    extract_event_type::bench_jmespath_2,
    extract_event_type::bench_accessor_1,
    extract_event_type::bench_accessor_2,
    extract_from_array::bench_jmespath,
    extract_from_array::bench_accessor,
    extract_from_map::bench_jmespath,
    extract_from_map::bench_accessor,
);

criterion_main!(benches);
