# Performance Tips

This section contains implementation details that could lead to better overall performance.
At the time of writing it is too early to figure out whether the entries in this list have a real impact on the code execution speed or resource usage; so, they are provided solely as suggestions to be further analyzed in a dedicated performance tuning phase.

Performance related notes:

- the event.created_ts field forces Accessor get() method to return a Cow<'o, str> instead of a &str. This is because the CreatedTsAccessor generates a new String instance. We could:
    - Change the created_ts type from u64 to String
    - Add a "_temp_vars" field to the Event in which we put all the generated objects whose lifetime should be bound to the lifetime of the event itself.
- The ProcessedEvent "matched" field is composed of two nested maps; consequently, to get an extracted var we have to look up two times. We could use a single level map where vars are concatenated with the rule name: e.g., rule_name.var_name
- Remove fern in favor of slog and use an async logger implementation (see: https://github.com/slog-rs/slog )
- SIMD?


Noteworthy libraries:
- https://github.com/Amanieu/hashbrown
- https://github.com/Amanieu/parking_lot
- https://docs.rs/chashmap/2.2.0/chashmap/


Useful tips:
- https://llogiq.github.io/2017/06/01/perf-pitfalls.html
-    When compiling on the target machine we can use CPU specific features: RUSTFLAGS='-C target-cpu=native' cargo build --release