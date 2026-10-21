# THEMIS-SMT-REPORTED-PARTITION [patch]

Target version: 0.11.3. Status: delivery blocked.
Owner: Codex task `01a15ee8-abb2-7558-91f5-d293b9d6a007`.
Branch: `fix/smt-reported-partition`.
Baseline: `5595fc0a5fdae7d9c017a26023f8960b8f8511fb` (connector and fresh clone).

## Contract and repair

ADR 0005 requires reported sibling membership to be a partition of
`0..logical_processors`. Reflexivity and reciprocity alone admit overlapping
cores. Sort/deduplicate each parsed list and require equality with every
referenced sibling's list. Self-membership plus equality along membership
proves reflexivity, symmetry and transitivity. Preserve existing bounds,
`None`, parser behavior and dense ids ordered by lowest processor.

## Checklist

- [x] [patch] Verify repository/default SHA, no open issues or PRs, isolated
  ownership, and applicable policy; preserve the existing clean owner checkout.
- [x] [patch] Reproduce chain `["0,1", "0,1,2", "1,2"]` yielding `[0,0,1]`
  against unchanged baseline production (new test only). Linux-target Miri:
  four existing sibling fixtures pass; regression fails with exit 101.
- [x] [patch] Add normalized-set validation, chain/star/cycle regressions,
  a nonuniform range/reorder/duplicate fixture, and an independent relation-law
  oracle checking that equal core ids mean exactly reported membership.
- [x] [patch] Complete supported-target checks and independent working-diff review;
  exact final commit attestation is retained in the task handoff.
- [ ] [patch] Pass intact delivery hooks before preparing a draft PR.

## Execution evidence and remaining gates

This host is AArch64 macOS with no installed Linux VM/container runtime.
Miri executes Linux-target fixtures under an interpreter; this is actual
fixture execution, separate from native Linux kernel execution. Mac-native
nextest excludes the Linux/Windows SMT modules. Cross-target checks only
compile source. Supplemental exhaustive algorithm checks execute extracted
production bodies on the Mac and do not claim repository Linux coverage.

No affinity, scheduler, provider, hook registration, credential, access or
security-setting changes belong to this repair. Merge, release and deployment
remain outside this pass. Exact final revision and gate results are recorded
in the task handoff.

Candidate validation: Mac native Nextest 78/78 and doctests 9/9; format, strict
Clippy in both feature modes, and warning-denied Rustdoc pass. Linux/Windows
all-target Clippy passes with testing and without default features (compilation
only). Linux-target Miri executes all 13 SMT tests, including the 69-relation
oracle in both textual forms. Rust/Cargo 1.81 library check passes on Mac.
Mac/Linux all-feature patch API checks pass 223 each against the exact baseline.
All 62 external lockfile package entries remain byte-equivalent as parsed data.

Independent complementary testing executes exact extracted production bodies:
8,330 directed cases through four processors and 67,734 symmetric cases through
six, mixed/range/duplicate fixtures and the maximum singleton snapshot all pass.
These checks supplement the repository tests and retain the execution
distinctions above. Independent static review reports no findings.

## Delivery blocker

The intact `.githooks/pre-push` refuses the proposed new
`refs/heads/fix/smt-reported-partition` on `ryancinsight/themis` at
`31d778894c4e5e8ca61f44f0c47ab5bcc9b9a44e` (exit 1):
"the trusted credential scanner is not reachable from this clone; the pushed
range was not scanned." Its lockfile/artifact checks also report unavailable
stack tools. The manual hook receives Git's exact ref-update input; no network
push is attempted. Delivery stops at this refusal; no alternative route, hook
registration change or bypass is used. The pre-commit hook passes with its
explicit standalone lockfile-check warning.

The following evidence-only commit records this blocker without changing
validated production or tests. Draft PR, native Linux/Windows CI, lockfile
integrity CI, nightly error-code/Miri jobs, and lifecycle acceptance remain
open. The existing old-Cargo developer-graph backlog remains separate.
