<a id="themis-cpu-type-report-validation"></a>
## THEMIS-CPU-TYPE-REPORT-VALIDATION — Reject partial CPU-type reports [patch] — review

- status: review; priority: correctness; target: 0.11.5.
- branch: `fix/cpu-type-report-validation`.
- owner: codex:01a17995-4c0b-701d-8625-8650165230c2.
- baseline: [18490a59](https://github.com/ryancinsight/themis/commit/18490a597dc4f2d2a04ad9f66e8d125fa16e037c).
- contract: a class table requires complete, disjoint CPU-type reports covering
  the snapshot; malformed nonempty lists are absence, empty lists claim no ids.
- basis: [ADR 0004](../docs/adr/0004-efficiency-class-is-an-ordinal.md) and
  [Linux v6.12 bitmap-list formatting](https://github.com/torvalds/linux/blob/v6.12/lib/vsprintf.c#L1169-L1195).
- [x] [patch] Reproduce three invalid-report acceptance failures natively.
- [x] [patch] Use the existing strict set parser without changing tolerant consumers.
- [x] [patch] Preserve empty types, all 256 eight-CPU partitions and capacity fallback.
- [x] [patch] Execute pure efficiency fixtures on macOS as well as backend test hosts.
- [ ] [patch] Complete exact-revision independent review and normal local/hosted gates.
- [ ] [patch] Verify conditional merge, package/source receipt and docs deployment.
- boundaries: no hook, affinity, SMT, dependency or public API changes.
- evidence: task `evidence/nov05-themis-cpu-types`; physical Linux hardware is
  not required to execute the pure parser, and no live malformed report is claimed.
