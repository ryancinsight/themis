# 1. Placement Hints

*Chapter prose deferred — DoR item.*

<!-- Key points to cover:
  - PlacementHint enum: Current, Numa(NumaNodeId), Domain(LocalityDomainId),
    Tier(MemoryTier), Any
  - Default is Current: the caller's current NUMA node is the lowest-latency
    choice in the absence of other information
  - Consumers (mnemosyne allocators, moirai worker selection) match on
    PlacementHint and resolve it against the detected topology
  - PlacementHint is Copy + Hash: safe to use in HashMap keys or to pass
    through task boundaries without cloning
  - The pattern: callers always express a *preference*, not a mandate; the
    allocator falls back gracefully if the preferred node is full
-->
