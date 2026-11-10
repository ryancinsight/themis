# Milestone ladder ([ADR 0007](../docs/adr/0007-release-scope-1-0.md))
<a id="M-topology-contracts"></a> **M-topology-contracts** (0.12.0) needs: (none) — topology and placement query contracts.
- exit: query CPU and NUMA topology contracts (`cargo test --locked --all-features --test topology` passes).
- exit: demonstrate placement hint usage (`cargo run --locked --example book_placement_hints --all-features` exits successfully).
- exit: resolve `THEMIS-CPU-TYPE-REPORT-VALIDATION` and `THEMIS-CACHE-REPORT-VALIDATION` with landed evidence on the default branch.
- standing: no open correctness item.
<a id="M-placement-binding"></a> **M-placement-binding** (0.13.0) needs: M-topology-contracts — branded placement and binding contracts.
- exit: exercise branded placement scopes (`cargo test --locked --all-features --test branded` passes).
- exit: demonstrate GPU/topology placement usage (`cargo run --locked --example book_gpu_topology --all-features` exits successfully).
- standing: no open correctness item.
<a id="M-1.0"></a> **M-1.0** (1.0.0) needs: M-placement-binding — stable placement-law release (authority-gated).
- standing: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked --all-features`, `cargo test --locked --doc --all-features`, and `cargo doc --locked --no-deps --all-features` pass; SemVer and publish dry-run gates required for the declared release pass; README and rustdoc are current; no open correctness item.
