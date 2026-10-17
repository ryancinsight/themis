<a id="themis-testing-darwin-visibility"></a>
## THEMIS-TESTING-DARWIN-VISIBILITY — Restore portable testing helpers [patch] — blocked

- status: blocked; owner: Codex; target: 0.11.2; priority: correctness.
- Baseline: macOS testing/all-features fails E0432; default/unit-only controls pass.
- Repair: align three helper guards with the existing testing feature contract.
- Default production guards, signatures, helper bodies and all tests stay intact.
- Evidence and invariants: [gap audit](../gap_audit.md#testing-helper-visibility-repair-0112-patch).
- [x] [patch] Existing cpu_topology harness: red before repair, 17/17 afterward.
- [x] [patch] Mac 78/78 debug/release, 21/21 core, strict diagnostics/docs; macOS CI added.
- [x] [patch] Linux/Windows/WASM/iOS checks and both locked API controls (196 each).
- [x] [patch] Rust/Cargo 1.81 library passes; old developer-graph limit tracked separately.
- [ ] [patch] Exact-head push/CI, package and full-history conformance assessed.
- [ ] [patch] Independent exact-head review and lifecycle acceptance.
- Merge, tags, publishing, credentials and service deployments remain held.
