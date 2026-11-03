# THEMIS-CACHE-REPORT-VALIDATION [patch]

- status: review; priority: correctness; target: 0.11.4.
- owner: codex:01a17995-4c0b-701d-8625-8650165230c2.
- branch: `fix/cache-report-validation`.
- baseline: [a421be51](https://github.com/ryancinsight/themis/commit/a421be512bf93c459cc9e6047d7bda466c3beb6e).
- contract: cache sharing is a complete provider-reported CPU set, or absence.
- reference: [Linux v6.12 bitmap-list emitter](https://github.com/torvalds/linux/blob/v6.12/lib/vsprintf.c#L1238-L1266)
  and [cache attribute emitter](https://github.com/torvalds/linux/blob/v6.12/drivers/base/cacheinfo.c#L588-L597).
- [x] [patch] Reproduce `0-1,broken` becoming a cache over `[0,1]`.
- [x] [patch] Reject malformed, overlapping, unordered and out-of-bound sets.
- [x] [patch] Preserve valid sparse sets, independent records and tolerant consumers.
- [x] [patch] Exercise the real filesystem reader and all 255 nonempty eight-CPU bitmaps.
- [ ] [patch] Finish independent final-revision review and unchanged local/hosted gates.
- [ ] [patch] Verify conditional merge, package/source receipt and deployment disposition.
- boundaries: no hook rollout, affinity/SMT relation, API or dependency changes.
- receipts: the task's `evidence/nov03-themis-cache` retains exact red/green
  commands, source identities and final delivery evidence. Mac fixtures are
  filesystem-reader coverage, not proof of Linux kernel integration.
