//! Integration tests: rc-data's cache: the life cycle and the round trip (docs/workflows/testing.md §3).
//! One module per former test file; run one with `cargo test-all --test data -- <module>::`.

mod lifecycle;
mod roundtrip;
