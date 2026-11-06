# THEMIS-SMT-COMPLETE-REPORT [patch]

- status: review; priority: correctness; target: 0.11.6.
- baseline: `f5fb5fcc048842807c4c38fa9929b52bf6c81571`.
- contract: a sibling partition is published only when every source token is
  valid and the resulting complete lists form the partition required by ADR 0005.
- [x] Reproduce a malformed suffix becoming a valid two-processor core.
- [x] Reject malformed, reversed, out-of-bound and empty suffixes after any range.
- [x] Preserve reordered, overlapping and duplicate valid ranges and existing
  partition tests; check range unions against an independent bitmap oracle.
- [ ] Complete final-revision tests, independent review, normal hooks and CI.
- [ ] Verify current registry version, release eligibility and deployment before
  any merge or publication.
- limits: parser fixtures do not establish that a live Linux kernel produced a
  malformed sibling report; native Linux and Windows CI remain required.
