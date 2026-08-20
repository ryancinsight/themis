# ADR 0003: Keep the branded region manifest thin

Status: Accepted

## Context

`src/branded/region/mod.rs` contains the `SyncRegionPlacement` capability,
NUMA-node distinctness validation, scope construction, and its unit tests. The
file is therefore an implementation module rather than a module manifest. It
was 481 lines at the fetched provider default, and the Atlas conformance scan
classified it as `manifest_implementation`.

## Decision

Move the capability implementation and its tests to `region/scope.rs`. Keep
`region/mod.rs` as the module manifest: it declares `cell`, `placement`,
`static_cell`, and the private `scope` module, then re-exports the two public
scope items. The public `themis` paths and the safety argument for duplicated
Melinoe tokens remain unchanged.

## Alternatives

Leaving the implementation in `mod.rs` preserves file locality but retains a
manifest that hides a bounded implementation concern and prevents the
conformance debt from decreasing. Splitting individual methods into several
files would add leaf modules without a second operation family; one focused
scope module is the smallest boundary that satisfies the current trigger.

## Verification

The provider format, locked all-target build, warning-denied Clippy, nextest,
doctests, Rustdoc, and Atlas conformance scan are the acceptance gates. The
conformance result must reduce `manifest_implementation` by one without any
other class increasing.
