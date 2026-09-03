//! Branded placement test harness.
//!
//! `scope` and `split` share `support`'s fixtures. They were two test
//! binaries, each pulling `support` in through `#[path]`, which compiled it
//! twice: the helpers only `split` calls were dead code in `scope`'s copy and
//! failed the `-D warnings` gate. One harness compiles the shared fixtures
//! once, with every caller present.
//!
//! Test code is exempt from `clippy::unwrap_used`: a panic here is the
//! failure report, not a shipped panic path.
#![allow(clippy::unwrap_used)]
#![expect(
    clippy::similar_names,
    reason = "index-suffixed pairs (permit0/permit1, cell0/cell1) name per-NUMA-node operands"
)]

mod scope;
mod split;
mod support;
