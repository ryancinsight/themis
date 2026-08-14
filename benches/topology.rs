#![expect(
    clippy::print_stdout,
    reason = "harness = false bench: stdout is how it reports timings"
)]

use std::hint::black_box;
use std::time::Instant;

use themis::CpuTopology;

const ITERATIONS: usize = 10_000;
const LOGICAL_PROCESSORS: usize = 64;

fn main() {
    let started = Instant::now();
    let mut mapped_processors = 0usize;

    for _ in 0..ITERATIONS {
        let topology = black_box(CpuTopology::single_node(LOGICAL_PROCESSORS));
        mapped_processors += black_box(topology.processor_node_pairs().count());
    }

    let elapsed = started.elapsed();
    let nanos_per_iteration = elapsed.as_nanos() / ITERATIONS as u128;
    println!(
        "topology_single_node_iter: {nanos_per_iteration} ns/iter ({mapped_processors} mapped processors)"
    );
}
