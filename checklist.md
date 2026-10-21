# Themis repair checklist

Target version: 0.11.3 [patch]

- [x] [patch] Reproduce overlapping SMT sibling reports against unchanged
  baseline production at `5595fc0a5fdae7d9c017a26023f8960b8f8511fb`.
- [x] [patch] Validate normalized sibling sets as a reported partition and
  preserve textual-order/duplicate compatibility and typed absence.
- [x] [patch] Complete supported-target checks and independent working-diff review.
- [ ] [patch] Satisfy intact delivery hooks before a draft PR.

Evidence and remaining gates: [SMT repair](backlog/themis-smt-reported-partition.md).
