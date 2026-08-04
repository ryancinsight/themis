# 5. CPU Topology

*Chapter prose deferred — DoR item.*

<!-- Key points to cover:
  - CpuTopology: snapshot of the NUMA layout, processor-to-node map,
    distance matrix, and detected cache levels
  - NumaNode: node ID, memory capacity, processor list
  - CacheLevel: L1/L2/L3 detected via CPUID or /sys/devices/system/cpu
  - detect() (std-only): reads the OS topology on the current host
  - CpuTopology::numa_node_for_processor(p): O(1) lookup of processor p's
    home NUMA node using the processor-to-node table
  - Distance matrix: inter-node access latency ratios; used by mnemosyne's
    remote-access fallback threshold
-->
