<a id="themis-msrv-cargo-dev-graph"></a>
## THEMIS-MSRV-CARGO-DEV-GRAPH — Assess the old Cargo developer graph [patch] — blocked

- status: blocked; owner: unclaimed; target: future; priority: correctness.
- Cargo 1.81 fetch/test of the locked dev graph fails parsing edition 2024 in
  serde_spanned 1.1.1; unchanged 0.11.1 and candidate 0.11.2 fail identically.
- Rust/Cargo 1.81 candidate library all-features check passes; its all-target
  developer tooling is a separate contract from the library compiler floor.
- All 62 external lock records stay identical; no downgrade or floor change.
- [x] [patch] Preserve baseline/candidate fetch failures and library controls.
- [ ] [patch] Assess intended developer Cargo policy before broader tooling claims.
- Details: [gap audit](../gap_audit.md#testing-helper-visibility-repair-0112-patch).
