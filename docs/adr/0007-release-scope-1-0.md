# ADR 0007: Themis release scope for 1.0

- Status: Accepted
- Date: 2026-11-10
- Class: `[arch]` `[minor]`

## Context

Themis (`themis-topology` 0.11.5) provides typed placement law for NUMA,
memory tiers, CPU topology, efficiency classes, thread binding, and branded
placement in the Atlas stack. The backlog contains open correctness and
structural items, and no milestone ladder exists to 1.0.

## Decision

Adopt a three-milestone ladder to 1.0 recorded in `backlog/milestones.md`:

- **M-topology-contracts** (0.12.0): users query and consume CPU/NUMA topology
  and placement contracts with validated reports.
- **M-placement-binding** (0.13.0): users obtain branded placement scopes and
  thread-binding behavior with verified contracts.
- **M-1.0** (1.0.0): stable placement-law surface (authority-gated) with
  documented contracts and release gates satisfied.
- **Toolchain floor:** `rust-version` 1.81.0 and edition 2021 hold through
  1.0. Either rises only when a non-dev dependency requires it, and the
  rise is recorded by a dated revision note on this ADR. Dev-dependencies,
  tests, benches and examples build on the pinned toolchain and do not set
  the floor (README, Standalone verification).

## Alternatives

- Keep roadmap only in backlog items: rejected because stack policy requires a
  runnable capability ladder.
- Delay ladder creation: rejected for a repository under multi-turn delivery.
- Raise the floor to 1.85 and the edition to 2024 now: rejected because no
  consumer or non-dev dependency need is on record, and it narrows the
  consumer set at the release that fixes the contract.

## Consequences

- Open items carry `milestone:` assignments and feature items name the exit
  check they serve.
- Milestones close only when exit checks pass on fetched default commits; 1.0
  release/publication remains authority-gated.
- This ADR is rewritten in place as scope evidence changes.

Revision 2026-11-28: record the `rust-version` floor, the edition and the
raise trigger (THEMIS-MSRV-FLOOR-ADR-2026-11-16); evidence: `Cargo.toml`
and `README.md` read at `0a6b020`.
