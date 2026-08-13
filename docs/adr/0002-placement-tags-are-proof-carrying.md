# ADR 0002: Placement tags carry their proof

Status: Accepted

## Context

`SyncRegionPlacement` wraps a Melinoe `SyncRegionToken<'brand>`. Melinoe's whole
guarantee is that at most one such token exists per brand, and that uniqueness
is what makes every `&mut T` obtained through a `MelinoeCell` unaliased.

The `split*` methods hand out several placement capabilities for one brand, so
they duplicate the token with `core::ptr::read`. After a split the token no
longer carries the exclusion. The design intended the **NUMA node tag** to carry
it instead: each capability accepts only pinned cells reporting its own tag, and
`split_static::<A, B>` proves `A != B` with a const assertion.

That argument had a hole. `A != B` proves the two *tags* differ; it says nothing
about the *cells* they reach. The tag was attached to a cell by the caller, at
the reference site, with no validation:

```rust
pub const fn ConstNumaPinnedCellRef::new(cell: &'a MelinoeCell<'brand, T>) -> Self
pub const fn NumaPinnedCellRef::new(node_id: NumaNodeId, cell: &'a MelinoeCell<'brand, T>) -> Self
```

Both take a shared `&MelinoeCell` and a tag chosen at the call site, so one cell
could be labelled twice. The exploit needed no `unsafe` from the caller:

```rust
let (mut p0, mut p1) = region.split_static::<0, 1>();
let r0 = ConstNumaPinnedCellRef::<0, _>::new(&cell);
let r1 = ConstNumaPinnedCellRef::<1, _>::new(&cell);
let m0 = p0.write(&r0);   // &mut u32
let m1 = p1.write(&r1);   // &mut u32 — same location
```

This compiled, ran, and observably aliased: a write through `m1` changed what
`m0` read. Miri reported a Stacked Borrows violation (`Unique retag`
invalidation) at the `DerefMut`. Themis was breaking Melinoe's contract in safe
code; Melinoe itself is sound.

## Decision

Bind the placement proof into the pinned-cell constructors, so a cell's tag
comes from a validated construction path instead of a caller argument.

1. **Delete the tag-accepting constructors.** `NumaPinnedCellRef::new`,
   `NumaPinnedSliceRef::new`, `ConstNumaPinnedCellRef::new`, and
   `ConstNumaPinnedSliceRef::new` are removed. There is no supported way to
   staple a caller-chosen tag onto a shared `&MelinoeCell`.

2. **Replace them with `from_unique`, taking `&'a mut`.** The exclusive borrow
   *is* the placement proof: it is consumed for `'a`, so the borrow checker
   rejects any second reference to the cell and therefore any second tag. This
   keeps the zero-allocation stack-array use case and needs no `unsafe`.

3. **Add `as_pinned_ref` projections** on the owning pinned types, which inherit
   the owner's tag rather than accepting one.

4. **Make the four pinned traits `unsafe trait`s.** `PinnedCell`,
   `ConstPinnedCell`, `PinnedSlice`, and `ConstPinnedSlice` are the dispatch
   surface the placement `write` methods use, so they are where the proof must
   be demanded. Their `# Safety` sections require that the returned cell be
   unreachable under any other node tag for the wrapper's lifetime. Without
   this, a downstream safe `impl` would reopen the hole; the seam stays open
   (per-consumer placement types remain implementable) at the cost of an
   explicit `unsafe impl`.

Ownership already discharged the obligation for the owning types
(`NumaPinnedCell` and friends mint their own `MelinoeCell`), which is why the
defect lived only in the borrowed wrappers.

Two consequences follow from stating the model precisely:

- `project_static` becomes **safe**. It consumes the region and returns a single
  capability, so it never duplicates the token and Melinoe's own exclusion
  carries the whole argument. Its `# Safety` clause described an obligation the
  signature already made impossible to violate.
- `split`/`split_with` assert pairwise-distinct node ids locally. `CpuTopology`
  already rejects duplicate ids when building its node index, so this cannot
  fire through the public API; it is here so the `unsafe` blocks discharge their
  precondition locally rather than depending on an assertion in another module.

## Alternatives

**Make `split`/`split_static` `unsafe`, stating disjointness as a caller
obligation.** Rejected. It closes the static hole by pushing an unbounded
obligation onto every caller, and it leaves the dynamic hole open:
`NumaPinnedCellRef::new(node_id, cell)` accepts an unvalidated `node_id`, so
both runtime node checks in `placement.rs` pass and two `NumaNodePlacement`s
alias just as readily. Marking the split unsafe would also mark the common,
genuinely-safe use as unsafe, which erodes the meaning of the keyword. The
chosen design keeps the safe API safe and closes both holes at their shared root
— the tag being a claim rather than a proof.

**Seal the four traits instead of making them `unsafe`.** Rejected. Sealing
forecloses downstream placement types, and the traits exist as an extension
seam. An open seam carrying a memory-safety contract is an `unsafe trait`.

**Runtime identity check (compare cell addresses against a per-capability
registry).** Rejected. It costs a lookup on every access to a zero-cost
capability API, and it detects the violation after construction rather than
making it unrepresentable.

## Verification

- The exploit was reproduced before the fix as a passing test, and under
  `cargo miri test` as a Stacked Borrows error.
- Two `compile_fail` doctests on `split_static` pin the outcome: the old
  sequence fails with **E0599** (the constructor is gone) and the `from_unique`
  replacement fails with **E0499** (borrow checker refuses the second tag).
- Error-code enforcement requires **nightly** rustdoc: stable silently ignores
  the `E0xxx` annotation on `compile_fail` blocks. CI therefore runs
  `cargo +nightly test --doc` in addition to the stable doctest pass, and the
  annotation was verified to fail when given a deliberately wrong code.
- Positive coverage asserts that disjoint capabilities still read and write
  their own cells and that projections inherit the owner's tag.
- `cargo miri test` over the branded module runs in CI.
