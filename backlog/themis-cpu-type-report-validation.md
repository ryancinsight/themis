<a id="themis-cpu-type-report-validation"></a>
## THEMIS-CPU-TYPE-REPORT-VALIDATION — Reject partial CPU-type reports [patch] — todo

- status: todo; priority: correctness; target: 0.11.5; branch: `fix/cpu-type-report-validation`.
- owner: codex:01a17995-4c0b-701d-8625-8650165230c2; final source review pending.
- baseline: [18490a59](https://github.com/ryancinsight/themis/commit/18490a597dc4f2d2a04ad9f66e8d125fa16e037c).
- contract: complete disjoint CPU-type reports cover the snapshot; malformed lists are absence, empty lists claim no ids.
- basis: [ADR 0004](../docs/adr/0004-efficiency-class-is-an-ordinal.md) and [Linux v6.12 emitter](https://github.com/torvalds/linux/blob/v6.12/lib/vsprintf.c#L1169-L1195).
- [x] [patch] Reproduce three invalid-report failures; preserve strict-set and tolerant-consumer separation.
- [x] [patch] Verify empty types, all 256 eight-CPU partitions, capacity fallback and all std test-host fixtures.
- [x] [patch] Pass 121 native tests, 9 doctests, Clippy, MSRV, packaging and 223 applicable SemVer checks.
- [ ] [patch] Complete final-revision independent review, normal hooks and hosted gates.
- [ ] [patch] Verify conditional merge, package/source receipt and docs deployment.
- boundaries: no hook, affinity, SMT, dependency or public API changes; no live malformed kernel report claimed.
- evidence: task `evidence/nov05-themis-cpu-types`; [root-cause audit](../gap_audit.md#cpu-type-report-validation-0115-patch).
