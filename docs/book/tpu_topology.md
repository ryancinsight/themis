# 7. TPU Topology

*Chapter prose deferred — DoR item.*

<!-- Key points to cover:
  - TpuTopology / TpuDeviceProperties: analogous to GpuTopology but for
    matrix-multiplication accelerators (Google TPU, Apple Neural Engine,
    Intel Gaudi)
  - matrix_units: matrix-multiplication-unit count (None when unreported)
  - systolic_width: systolic-array width in elements (None when unreported)
  - memory_bytes: device memory capacity (None when unreported)
  - Usage: moirai's task router uses TpuTopology to decide whether to route
    a `coeus` tensor operation to a TPU device
-->
