# themis gap audit

## Incremental release baseline

Reviewed [themis-topology 0.11.5](Cargo.toml) at [8a38eb37152f](https://github.com/ryancinsight/themis/tree/8a38eb37152f608b46b627b225491b4d5c809103). The exact version appears in the [registry](https://crates.io/crates/themis-topology/0.11.5).

The next [minor] checkpoint uses [adjacency properties](tests/topology/cpu/adjacency_properties.rs). [Target version and acceptance criteria](backlog.md#themis-stable-qualification).

## Open risks

### GATE-FEATURE-GATED-HARNESS-2026-09-21 — a default-feature gate skips two harnesses

`branded` carries `required-features = ["melinoe", "testing"]` and
`cpu_topology` carries `testing`, so a local `cargo nextest run` without
them reports a clean 77/77 while CI, which passes `--features testing`,
runs 119 and was red for three days on a compile error in `branded`
([#54](https://github.com/ryancinsight/themis/pull/54)). The closed
THE-WASM-2026-09-06 block recorded `77/77` for the
same reason, so the gap has been mistaken for a pass more than once.

The stack's `[patch]` overlay compounds it: under the atlas umbrella
melinoe resolves to the local tree, so even a feature-complete run there
cannot see a stale pin. Only a standalone resolution reproduces CI.

**Re-open trigger:** any local gate claim for this member that does not
name `--features testing` and does not run outside the overlay.

**Mechanization:** the member needs a committed gate script that both the
pre-push hook and CI invoke, so the feature set cannot diverge between
them (`engineering_gates`: one gate definition).

### THEMIS-NODE-PRIMARY-GROUP-2026-11-13 — a Windows node past 64 processors reports one group

`detect/windows.rs` takes one `GROUP_AFFINITY` per node from `GetNumaNodeProcessorMaskEx`, which
returns only the primary group since "TBD Release Iron" ([Remarks][gnnpm]); a node past the group
size spans several ([Processor Groups][pg]), so the rest go unreported (cure: [RelationNumaNodeEx][glpie]).

**Re-open trigger:** a Windows host with a node of over 64 logical processors, or a consumer binding it.

[gnnpm]: https://learn.microsoft.com/en-us/windows/win32/api/systemtopologyapi/nf-systemtopologyapi-getnumanodeprocessormaskex
[pg]: https://learn.microsoft.com/en-us/windows/win32/procthread/processor-groups
[glpie]: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex
