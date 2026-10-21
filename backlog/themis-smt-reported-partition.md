# THEMIS-SMT-REPORTED-PARTITION [patch]

- status: review; priority: correctness; owner: Codex; target: 0.11.3.
- branch: `fix/smt-reported-partition`.
- baseline: [verified standalone main](https://github.com/ryancinsight/themis/commit/5595fc0a5fdae7d9c017a26023f8960b8f8511fb).
- contract: ADR 0005 requires reported sibling membership to form a partition.
- proof: self-membership and equal normalized rows along membership imply symmetry and transitivity.
- [x] [patch] Reproduce reciprocal chain reports yielding `[0,0,1]` instead of absence.
- [x] [patch] Reject chain/star/cycle overlaps; preserve bounds, typed absence, ordering and duplicates.
- [x] [patch] Check accepted core ids against independent equivalence laws and reported membership.
- evidence: Linux-target Miri 13/13; native Mac Nextest 78/78 and doctests 9/9; strict diagnostics pass.
- compatibility: Rust 1.81 Mac library and Mac/Linux API checks pass; external dependencies stay unchanged.
- independent review and exhaustive extracted-algorithm checks pass; these Mac runs are not native Linux coverage.
- delivery: intact hook, native platform CI and lifecycle acceptance remain required before merge.
- receipts: task handoff retains exact revisions, full results, original scanner refusal and registered-lane validation.
