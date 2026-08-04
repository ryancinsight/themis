# 3. Locality Identities

*Chapter prose deferred — DoR item.*

<!-- Key points to cover:
  - NumaNodeId: repr(transparent) u32; ZERO and INVALID sentinels; is_valid();
    bucket_index::<N>() const method
  - NumaBucketIndex<N>: const-generic; wrapping_add wraps within the table;
    ASSERT_NONZERO catches zero-bucket tables at compile time
  - WorkerId: repr(transparent) u32; INVALID sentinel; is_valid()
  - LocalityDomainId: coarser than NUMA node; represents a group of NUMA nodes
    with the same memory-distance law (e.g. one NUMA domain per socket)
  - TopologyEpoch: invalidation token; when topology changes (hot-add node,
    GPU insertion), the epoch advances and cached topology snapshots are stale
-->
