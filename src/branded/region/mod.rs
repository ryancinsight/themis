//! Thread-portable branded placement scopes.

pub mod cell;
pub mod placement;
pub mod static_cell;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::vec::Vec;

use melinoe::sync::{sync_region_scope, SyncRegionToken};

/// Thread-portable placement-region capability.
///
/// The inner Melinoe token is `Send + Sync`, so a complete placement region may
/// be moved between execution domains while preserving single-writer discipline.
///
/// # How splitting stays sound
///
/// Melinoe grants one write token per brand, and that uniqueness is what makes
/// every `&mut T` obtained through a [`MelinoeCell`](melinoe::MelinoeCell)
/// unaliased. The `split*` methods below hand out several capabilities for one
/// brand, so they duplicate the token: after a split the token no longer
/// carries the exclusion.
///
/// What carries it instead is the **NUMA node tag**. Each capability accepts
/// only pinned cells reporting its own tag, and the
/// [`PinnedCell`](cell::PinnedCell) family of traits obliges every pinned
/// wrapper to be reachable under exactly one tag — an obligation their
/// constructors discharge by ownership or by an exclusive borrow, never by
/// accepting a tag as an argument. Distinct tags therefore name disjoint sets
/// of cells, and no two capabilities can produce `&mut T` to one location.
///
/// Both halves of that argument are load-bearing. The tag distinctness alone is
/// not enough: it was the missing cell-side half that once let two placements
/// address one cell from entirely safe code.
pub struct SyncRegionPlacement<'brand> {
    token: SyncRegionToken<'brand>,
}

/// Hard cap on NUMA node ids, mirroring `topology::cpu::tables`.
const MAX_NUMA_NODE_IDS: usize = 1024;

/// Verifies that a topology's NUMA node ids are pairwise distinct.
///
/// [`SyncRegionPlacement::split`] mints one capability per node and its `unsafe`
/// token duplication relies on their tags differing; two capabilities sharing a
/// tag would both accept the same pinned cells and could hand out overlapping
/// `&mut T`.
///
/// `CpuTopology` already rejects duplicate ids when it builds its node index, so
/// this cannot fire through the public API. It is here so the `unsafe` blocks in
/// `split` and `split_with` discharge their precondition locally instead of
/// depending on an assertion in a different module — one linear pass over a
/// table that is at most [`MAX_NUMA_NODE_IDS`] long.
fn assert_distinct_node_ids(nodes: &[crate::NumaNode]) {
    let mut seen = [0u64; MAX_NUMA_NODE_IDS / u64::BITS as usize];
    for node in nodes {
        let index = node.id.index();
        assert!(
            index < MAX_NUMA_NODE_IDS,
            "invariant: NUMA node id {index} exceeds the {MAX_NUMA_NODE_IDS}-node cap"
        );
        let (word, bit) = (index / u64::BITS as usize, index % u64::BITS as usize);
        assert!(
            seen[word] & (1 << bit) == 0,
            "invariant: CpuTopology NUMA node ids must be pairwise distinct; \
             duplicate id {index} would give two placement capabilities the same \
             tag and let them alias one pinned cell"
        );
        seen[word] |= 1 << bit;
    }
}

impl<'brand> SyncRegionPlacement<'brand> {
    /// Creates a Melinoe cell in this placement brand.
    #[must_use]
    #[inline]
    pub const fn cell<T>(&self, value: T) -> melinoe::MelinoeCell<'brand, T> {
        melinoe::MelinoeCell::new(value)
    }

    /// Reads placement state through the sync-region permit.
    #[inline]
    pub fn read<'a, T>(
        &'a self,
        cell: &'a melinoe::MelinoeCell<'brand, T>,
    ) -> melinoe::MelinoeRef<'a, 'brand, T> {
        cell.borrow(&self.token)
    }

    /// Writes placement state through the sync-region permit.
    #[inline]
    pub fn write<'a, T>(
        &'a mut self,
        cell: &'a melinoe::MelinoeCell<'brand, T>,
    ) -> melinoe::MelinoeMut<'a, 'brand, T> {
        cell.borrow_mut(&mut self.token)
    }

    /// Splits this global capability into per-node capabilities based on the CPU topology.
    ///
    /// Each returned capability writes only the pinned cells reporting its own
    /// NUMA node id; see [the soundness argument](Self#how-splitting-stays-sound).
    ///
    /// # Panics
    ///
    /// Panics if `topology` reports two NUMA nodes with the same id. Distinct
    /// ids are what keep the returned capabilities disjoint, so a malformed
    /// topology is refused rather than silently turned into aliasing.
    /// `CpuTopology` construction already rejects duplicate ids, so this is the
    /// local discharge of that precondition rather than a reachable failure.
    #[must_use]
    pub fn split(self, topology: &crate::CpuTopology) -> Vec<placement::NumaNodePlacement<'brand>> {
        let nodes = topology.numa_nodes();
        assert_distinct_node_ids(nodes);

        let this = core::mem::ManuallyDrop::new(self);
        let mut split = Vec::with_capacity(nodes.len());
        for node in nodes {
            // SAFETY: `this` is a `ManuallyDrop`, so the source token stays live
            // across every read and is never dropped; each `ptr::read` therefore
            // yields the sole owner of its bits.
            //
            // Duplicating the token suspends Melinoe's one-token-per-brand
            // exclusion, so disjointness is re-established on the cell side:
            // `assert_distinct_node_ids` above proves the tags are pairwise
            // distinct, and the `PinnedCell` contract proves each pinned cell
            // answers to exactly one tag. The capabilities therefore reach
            // disjoint cells and cannot alias `&mut T`.
            unsafe {
                split.push(placement::NumaNodePlacement {
                    node_id: node.id,
                    token: core::ptr::read(&raw const this.token),
                });
            }
        }
        split
    }

    /// Splits this global capability into per-node capabilities using a callback,
    /// avoiding heap allocations on topologies with up to 128 nodes.
    ///
    /// # Panics
    ///
    /// Panics if `topology` reports two NUMA nodes with the same id, for the
    /// same reason as [`split`](Self::split).
    pub fn split_with<F, R>(self, topology: &crate::CpuTopology, f: F) -> R
    where
        F: FnOnce(&mut [placement::NumaNodePlacement<'brand>]) -> R,
    {
        const MAX_STACK_NODES: usize = 128;

        let nodes = topology.numa_nodes();
        assert_distinct_node_ids(nodes);

        let this = core::mem::ManuallyDrop::new(self);
        let num_nodes = nodes.len();

        if num_nodes <= MAX_STACK_NODES {
            struct DropGuard<'brand> {
                ptr: *mut placement::NumaNodePlacement<'brand>,
                initialized: usize,
            }
            impl Drop for DropGuard<'_> {
                fn drop(&mut self) {
                    for i in 0..self.initialized {
                        // SAFETY: `ptr` is the base of the `MaybeUninit` array
                        // below and `initialized` counts exactly the prefix the
                        // loop wrote, so for `i < initialized` the offset is in
                        // bounds and addresses an initialized, owned
                        // `NumaNodePlacement`. The array is never read after
                        // this guard runs, so nothing drops these slots twice.
                        unsafe {
                            core::ptr::drop_in_place(self.ptr.add(i));
                        }
                    }
                }
            }

            let mut buf = core::mem::MaybeUninit::<
                [placement::NumaNodePlacement<'brand>; MAX_STACK_NODES],
            >::uninit();
            let buf_ptr = buf
                .as_mut_ptr()
                .cast::<placement::NumaNodePlacement<'brand>>();
            let mut guard = DropGuard {
                ptr: buf_ptr,
                initialized: 0,
            };

            for (i, node) in nodes.iter().enumerate() {
                // SAFETY: this branch runs only when `num_nodes <=
                // MAX_STACK_NODES`, and `i < num_nodes`, so `buf_ptr.add(i)` is
                // inside `buf`. `write` initializes the slot without dropping
                // the uninitialized value it replaces, and `guard.initialized`
                // is advanced immediately after so an unwind drops exactly the
                // slots written so far.
                //
                // The token read is sound for the reason given in `split`:
                // `this` is a `ManuallyDrop` that outlives every read. The
                // duplication is disjoint by the same distinct-tag plus
                // `PinnedCell` argument, with distinctness asserted above.
                unsafe {
                    buf_ptr.add(i).write(placement::NumaNodePlacement {
                        node_id: node.id,
                        token: core::ptr::read(&raw const this.token),
                    });
                }
                guard.initialized += 1;
            }

            // SAFETY: the loop above initialized exactly `num_nodes`
            // consecutive slots from `buf_ptr`, and `num_nodes <=
            // MAX_STACK_NODES` keeps the range inside `buf`, which outlives the
            // borrow handed to `f`. `guard` holds the only other pointer to
            // this memory and touches it solely in `Drop`, which runs after `f`
            // returns, so the `&mut` is unaliased for its whole life.
            let slice = unsafe { core::slice::from_raw_parts_mut(buf_ptr, num_nodes) };
            f(slice)
        } else {
            // Fallback to heap allocation if we exceed stack limits
            let mut split = Vec::with_capacity(num_nodes);
            for node in nodes {
                // SAFETY: as in `split` — `this` is a `ManuallyDrop` that
                // outlives every read, so each `ptr::read` yields the sole owner
                // of its bits, and the distinct-tag assertion above plus the
                // `PinnedCell` contract keep the duplicated tokens' reachable
                // cell sets disjoint.
                unsafe {
                    split.push(placement::NumaNodePlacement {
                        node_id: node.id,
                        token: core::ptr::read(&raw const this.token),
                    });
                }
            }
            f(&mut split)
        }
    }

    /// Splits this global capability into two disjoint compile-time checked node capabilities.
    ///
    /// `A != B` is enforced at compile time. That makes the two capabilities'
    /// *tags* distinct; the [`ConstPinnedCell`](cell::ConstPinnedCell) contract
    /// supplies the other half of the argument, that a cell answers to exactly
    /// one tag. See [the soundness argument](Self#how-splitting-stays-sound).
    ///
    /// A cell can no longer be labelled with two node ids, so the sequence that
    /// once produced two live `&mut T` to one location does not compile. The
    /// constructor it relied on is gone:
    ///
    /// ```compile_fail,E0599
    /// use themis::{sync_region_placement_scope, ConstNumaPinnedCellRef};
    ///
    /// sync_region_placement_scope(|region| {
    ///     let cell = region.cell(0u32);
    ///     let (mut p0, mut p1) = region.split_static::<0, 1>();
    ///     // No `new`: a tag cannot be attached to a shared `&MelinoeCell`.
    ///     let r0 = ConstNumaPinnedCellRef::<0, _>::new(&cell);
    ///     let r1 = ConstNumaPinnedCellRef::<1, _>::new(&cell);
    ///     *p0.write(&r0) = 1;
    ///     *p1.write(&r1) = 2;
    /// });
    /// ```
    ///
    /// and its replacement is checked by the borrow checker, which refuses the
    /// second tag because the first still holds the cell exclusively:
    ///
    /// ```compile_fail,E0499
    /// use themis::{sync_region_placement_scope, ConstNumaPinnedCellRef};
    ///
    /// sync_region_placement_scope(|region| {
    ///     let mut cell = region.cell(0u32);
    ///     let (mut p0, mut p1) = region.split_static::<0, 1>();
    ///     let r0 = ConstNumaPinnedCellRef::<0, _>::from_unique(&mut cell);
    ///     let r1 = ConstNumaPinnedCellRef::<1, _>::from_unique(&mut cell);
    ///     *p0.write(&r0) = 1;
    ///     *p1.write(&r1) = 2;
    /// });
    /// ```
    #[must_use]
    #[inline]
    pub fn split_static<const A: u32, const B: u32>(
        self,
    ) -> (
        placement::ConstNumaNodePlacement<'brand, A>,
        placement::ConstNumaNodePlacement<'brand, B>,
    ) {
        struct AssertDisjoint<const A: u32, const B: u32>;
        impl<const A: u32, const B: u32> AssertDisjoint<A, B> {
            const OK: () = {
                assert!(A != B, "Static NUMA node split must be disjoint");
            };
        }
        let () = AssertDisjoint::<A, B>::OK;

        let this = core::mem::ManuallyDrop::new(self);
        // SAFETY: `this` is a `ManuallyDrop`, so the source token stays live
        // across both reads and is never dropped; each `ptr::read` yields the
        // sole owner of its bits.
        //
        // The duplication is disjoint because `A != B` is asserted above at
        // compile time and, by the `ConstPinnedCell` contract, a cell reachable
        // as `ConstPinnedCell<'brand, A, _>` is not reachable as
        // `ConstPinnedCell<'brand, B, _>`. The two capabilities address disjoint
        // cells, so neither can observe the other's `&mut T`.
        unsafe {
            (
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&raw const this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&raw const this.token),
                },
            )
        }
    }

    /// Splits this global capability into three disjoint compile-time checked node capabilities.
    ///
    /// The distinctness of `A`, `B`, and `C` is enforced at compile time, so two
    /// capabilities can never alias the same NUMA node.
    #[must_use]
    #[inline]
    pub fn split_static_3<const A: u32, const B: u32, const C: u32>(
        self,
    ) -> (
        placement::ConstNumaNodePlacement<'brand, A>,
        placement::ConstNumaNodePlacement<'brand, B>,
        placement::ConstNumaNodePlacement<'brand, C>,
    ) {
        struct AssertDisjoint<const A: u32, const B: u32, const C: u32>;
        impl<const A: u32, const B: u32, const C: u32> AssertDisjoint<A, B, C> {
            const OK: () = {
                assert!(
                    A != B && A != C && B != C,
                    "Static NUMA node split must be disjoint"
                );
            };
        }
        let () = AssertDisjoint::<A, B, C>::OK;

        let this = core::mem::ManuallyDrop::new(self);
        // SAFETY: `this` is a `ManuallyDrop`, so the source token stays live
        // across all three reads and is never dropped; each `ptr::read` yields
        // the sole owner of its bits. `A`, `B`, and `C` are pairwise distinct by
        // the compile-time assertion above, and the `ConstPinnedCell` contract
        // makes each cell reachable under exactly one tag, so the three
        // capabilities address disjoint cells.
        unsafe {
            (
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&raw const this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&raw const this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&raw const this.token),
                },
            )
        }
    }

    /// Splits this global capability into four disjoint compile-time checked node capabilities.
    ///
    /// The distinctness of `A`, `B`, `C`, and `D` is enforced at compile time, so
    /// two capabilities can never alias the same NUMA node.
    #[must_use]
    #[inline]
    pub fn split_static_4<const A: u32, const B: u32, const C: u32, const D: u32>(
        self,
    ) -> (
        placement::ConstNumaNodePlacement<'brand, A>,
        placement::ConstNumaNodePlacement<'brand, B>,
        placement::ConstNumaNodePlacement<'brand, C>,
        placement::ConstNumaNodePlacement<'brand, D>,
    ) {
        struct AssertDisjoint<const A: u32, const B: u32, const C: u32, const D: u32>;
        impl<const A: u32, const B: u32, const C: u32, const D: u32> AssertDisjoint<A, B, C, D> {
            const OK: () = {
                assert!(
                    A != B && A != C && A != D && B != C && B != D && C != D,
                    "Static NUMA node split must be disjoint"
                );
            };
        }
        let () = AssertDisjoint::<A, B, C, D>::OK;

        let this = core::mem::ManuallyDrop::new(self);
        // SAFETY: `this` is a `ManuallyDrop`, so the source token stays live
        // across all four reads and is never dropped; each `ptr::read` yields
        // the sole owner of its bits. `A`, `B`, `C`, and `D` are pairwise
        // distinct by the compile-time assertion above, and the
        // `ConstPinnedCell` contract makes each cell reachable under exactly one
        // tag, so the four capabilities address disjoint cells.
        unsafe {
            (
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&raw const this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&raw const this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&raw const this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&raw const this.token),
                },
            )
        }
    }

    /// Projects this global capability into a static node capability.
    ///
    /// Unlike the `split*` methods this does not duplicate the token: it
    /// consumes the region and returns a single capability, so the brand still
    /// has exactly one writer and Melinoe's own exclusion carries the whole
    /// argument. No disjointness obligation falls on the caller, which is why
    /// this is safe.
    #[must_use]
    #[inline]
    pub fn project_static<const NODE_ID: u32>(
        self,
    ) -> placement::ConstNumaNodePlacement<'brand, NODE_ID> {
        let this = core::mem::ManuallyDrop::new(self);
        placement::ConstNumaNodePlacement {
            // SAFETY: `this` is a `ManuallyDrop`, so the source token remains
            // live and is never dropped; the read yields the sole owner
            // of its bits. `self` was consumed and only one capability leaves
            // this function, so the brand retains exactly one live token.
            token: unsafe { core::ptr::read(&raw const this.token) },
        }
    }
}

/// Opens a thread-portable placement-region scope.
#[inline]
pub fn sync_region_placement_scope<R>(
    f: impl for<'brand> FnOnce(SyncRegionPlacement<'brand>) -> R,
) -> R {
    sync_region_scope(|token| f(SyncRegionPlacement { token }))
}

#[cfg(test)]
mod tests {
    use super::assert_distinct_node_ids;
    use crate::{MemoryTier, NumaNode, NumaNodeId};

    fn node(id: u32) -> NumaNode {
        NumaNode {
            id: NumaNodeId::new(id),
            processors: Box::new([id]),
            distances: Box::new([10]),
            memory_tier: MemoryTier::Dram,
        }
    }

    #[test]
    fn distinct_node_ids_are_accepted() {
        assert_distinct_node_ids(&[node(0), node(1), node(7), node(1023)]);
    }

    #[test]
    #[should_panic(expected = "pairwise distinct")]
    fn repeated_node_id_is_rejected() {
        assert_distinct_node_ids(&[node(0), node(1), node(0)]);
    }

    #[test]
    #[should_panic(expected = "exceeds the 1024-node cap")]
    fn out_of_range_node_id_is_rejected() {
        assert_distinct_node_ids(&[node(1024)]);
    }
}
