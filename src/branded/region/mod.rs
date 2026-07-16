//! Thread-portable branded placement scopes.

pub mod cell;
pub mod placement;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::vec::Vec;

use melinoe::sync::{sync_region_scope, SyncRegionToken};

/// Thread-portable placement-region capability.
///
/// The inner Melinoe token is `Send + Sync`, so a complete placement region may
/// be moved between execution domains while preserving single-writer discipline.
pub struct SyncRegionPlacement<'brand> {
    token: SyncRegionToken<'brand>,
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
    #[must_use]
    pub fn split(self, topology: &crate::CpuTopology) -> Vec<placement::NumaNodePlacement<'brand>> {
        let this = core::mem::ManuallyDrop::new(self);
        let nodes = topology.numa_nodes();
        let mut split = Vec::with_capacity(nodes.len());
        for node in nodes {
            // SAFETY: We consume `self` via `ManuallyDrop`, and we duplicate `this.token`
            // (which is a ZST/copy-safe type inside melinoe) for each unique NUMA node.
            // Since node IDs are unique in a topology, no two NumaNodePlacements will
            // have the same node_id, preventing concurrent writes to the same NumaPinnedCell.
            unsafe {
                split.push(placement::NumaNodePlacement {
                    node_id: node.id,
                    token: core::ptr::read(&this.token),
                });
            }
        }
        split
    }

    /// Splits this global capability into per-node capabilities using a callback,
    /// avoiding heap allocations on topologies with up to 128 nodes.
    pub fn split_with<F, R>(self, topology: &crate::CpuTopology, f: F) -> R
    where
        F: FnOnce(&mut [placement::NumaNodePlacement<'brand>]) -> R,
    {
        let this = core::mem::ManuallyDrop::new(self);
        let nodes = topology.numa_nodes();
        let num_nodes = nodes.len();

        const MAX_STACK_NODES: usize = 128;
        if num_nodes <= MAX_STACK_NODES {
            struct DropGuard<'brand> {
                ptr: *mut placement::NumaNodePlacement<'brand>,
                initialized: usize,
            }
            impl<'brand> Drop for DropGuard<'brand> {
                fn drop(&mut self) {
                    for i in 0..self.initialized {
                        unsafe {
                            core::ptr::drop_in_place(self.ptr.add(i));
                        }
                    }
                }
            }

            let mut buf = core::mem::MaybeUninit::<
                [placement::NumaNodePlacement<'brand>; MAX_STACK_NODES],
            >::uninit();
            let buf_ptr = buf.as_mut_ptr() as *mut placement::NumaNodePlacement<'brand>;
            let mut guard = DropGuard {
                ptr: buf_ptr,
                initialized: 0,
            };

            for (i, node) in nodes.iter().enumerate() {
                unsafe {
                    buf_ptr.add(i).write(placement::NumaNodePlacement {
                        node_id: node.id,
                        token: core::ptr::read(&this.token),
                    });
                }
                guard.initialized += 1;
            }

            // Create a slice from the initialized prefix
            let slice = unsafe { core::slice::from_raw_parts_mut(buf_ptr, num_nodes) };
            f(slice)
        } else {
            // Fallback to heap allocation if we exceed stack limits
            let mut split = Vec::with_capacity(num_nodes);
            for node in nodes {
                unsafe {
                    split.push(placement::NumaNodePlacement {
                        node_id: node.id,
                        token: core::ptr::read(&this.token),
                    });
                }
            }
            f(&mut split)
        }
    }

    /// Splits this global capability into two disjoint compile-time checked node capabilities.
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
        unsafe {
            (
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&this.token),
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
        unsafe {
            (
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&this.token),
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
        unsafe {
            (
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&this.token),
                },
                placement::ConstNumaNodePlacement {
                    token: core::ptr::read(&this.token),
                },
            )
        }
    }

    /// Projects this global capability into a static node capability.
    ///
    /// # Safety
    /// The caller must ensure that no other capability for the same `NODE_ID`
    /// is created or used concurrently under this brand.
    #[must_use]
    #[inline]
    pub unsafe fn project_static<const NODE_ID: u32>(
        self,
    ) -> placement::ConstNumaNodePlacement<'brand, NODE_ID> {
        let this = core::mem::ManuallyDrop::new(self);
        placement::ConstNumaNodePlacement {
            token: unsafe { core::ptr::read(&this.token) },
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
