# 3. Locality Identities

Themis provides four newtype wrappers that carry locality identity through the
Atlas stack without ambiguity.  All four live in `src/law/identity.rs` and
are `Copy + Eq + Hash`.

## `NumaNodeId`

```rust,ignore
pub struct NumaNodeId(u32);
impl NumaNodeId {
    pub const ZERO: Self;
    pub const INVALID: Self; // u32::MAX
    pub fn is_valid(self) -> bool;
    pub fn bucket_index<const N: usize>(self) -> NumaBucketIndex<N>;
}
```

`NumaNodeId` names a single NUMA node as reported by the OS.  The sentinel
`INVALID` (`u32::MAX`) is returned by APIs that could not resolve a node;
`is_valid()` returns `false` for it.  `ZERO` is the first physical node and
is always present on single-socket hosts.

`bucket_index::<N>()` maps an arbitrary node id into a fixed-size placement
table of `N` buckets using `id % N`.  This allows work-stealing domains and
allocation tables to be indexed in O(1) without a full topology lookup.

## `NumaBucketIndex<const BUCKETS: usize>`

```rust,ignore
pub struct NumaBucketIndex<const BUCKETS: usize>(usize);
impl<const BUCKETS: usize> NumaBucketIndex<BUCKETS> {
    pub const ASSERT_NONZERO: ();
    pub fn wrapping_add(self, rhs: usize) -> Self;
}
```

A const-generic index into a table of exactly `BUCKETS` entries.
`ASSERT_NONZERO` is a compile-time assertion that fires if `BUCKETS == 0`;
it should be evaluated in a `const _: () = NumaBucketIndex::<N>::ASSERT_NONZERO`
at the definition site of the table.

`wrapping_add` increments the index modulo `BUCKETS`, making it safe to
advance a cursor through the table without bounds checks.

## `WorkerId`

```rust,ignore
pub struct WorkerId(u32);
impl WorkerId {
    pub const INVALID: Self; // u32::MAX
    pub fn is_valid(self) -> bool;
}
```

An opaque identifier for a moirai worker thread or task-execution slot.
`INVALID` is the sentinel for "not yet assigned" or "unresolvable".  The
moirai scheduler assigns valid `WorkerId` values from its pool; callers that
receive `INVALID` should fall back to unaffinitised task submission.

## `LocalityDomainId`

```rust,ignore
pub struct LocalityDomainId(u32);
impl LocalityDomainId {
    pub const INVALID: Self; // u32::MAX
    pub fn is_valid(self) -> bool;
}
```

`LocalityDomainId` is coarser than `NumaNodeId`.  A locality domain groups
one or more NUMA nodes that share the same memory-distance law — typically all
nodes on a single CPU socket.  Callers that want socket-local placement but do
not need sub-socket precision use `PlacementHint::Domain(id)` rather than
`PlacementHint::Numa(id)`.

## Sentinels and validity

All four types share the same sentinel pattern: `INVALID = u32::MAX`,
`is_valid()` returns `false` for the sentinel.  This avoids magic-number
comparisons at call sites and makes validity checks self-documenting.  The
sentinel value is not zero, so zero-initialised memory does not accidentally
produce an invalid id — `NumaNodeId::ZERO` is a meaningful node.

## Relationship to topology

`NumaNodeId` and `LocalityDomainId` are populated by
[`CpuTopology`](cpu_topology.md).  `WorkerId` is assigned by moirai; themis
only owns the type.  [`PlacementHint`](placement_hints.md) carries all three
as arguments to its `Numa` and `Domain` variants.
