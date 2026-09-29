//! Integration tests: the extractor: synthetic discs, and the by-hand golden run against a real disc (docs/workflows/testing.md §3).
//! One module per former test file; run one with `cargo test-all --test extract -- <module>::`.

mod golden;
mod synthetic;
