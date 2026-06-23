//! Thread-portable branded placement scopes.

use melinoe::sync::{sync_region_scope, SyncRegionToken};
use melinoe::{MelinoeCell, MelinoeMut, MelinoeRef};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};

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
    pub const fn cell<T>(&self, value: T) -> MelinoeCell<'brand, T> {
        MelinoeCell::new(value)
    }

    /// Reads placement state through the sync-region permit.
    #[inline]
    pub fn read<'a, T>(&'a self, cell: &'a MelinoeCell<'brand, T>) -> MelinoeRef<'a, 'brand, T> {
        cell.borrow(&self.token)
    }

    /// Writes placement state through the sync-region permit.
    #[inline]
    pub fn write<'a, T>(
        &'a mut self,
        cell: &'a MelinoeCell<'brand, T>,
    ) -> MelinoeMut<'a, 'brand, T> {
        cell.borrow_mut(&mut self.token)
    }

    /// Splits this global capability into per-node capabilities based on the CPU topology.
    #[must_use]
    pub fn split(self, topology: &crate::CpuTopology) -> Vec<NumaNodePlacement<'brand>> {
        let this = core::mem::ManuallyDrop::new(self);
        let nodes = topology.numa_nodes();
        let mut split = Vec::with_capacity(nodes.len());
        for node in nodes {
            // SAFETY: We consume `self` via `ManuallyDrop`, and we duplicate `this.token`
            // (which is a ZST/copy-safe type inside melinoe) for each unique NUMA node.
            // Since node IDs are unique in a topology, no two NumaNodePlacements will
            // have the same node_id, preventing concurrent writes to the same NumaPinnedCell.
            unsafe {
                split.push(NumaNodePlacement {
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
        F: FnOnce(&mut [NumaNodePlacement<'brand>]) -> R,
    {
        let this = core::mem::ManuallyDrop::new(self);
        let nodes = topology.numa_nodes();
        let num_nodes = nodes.len();

        const MAX_STACK_NODES: usize = 128;
        if num_nodes <= MAX_STACK_NODES {
            struct DropGuard<'brand> {
                ptr: *mut NumaNodePlacement<'brand>,
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

            let mut buf = core::mem::MaybeUninit::<[NumaNodePlacement<'brand>; MAX_STACK_NODES]>::uninit();
            let buf_ptr = buf.as_mut_ptr() as *mut NumaNodePlacement<'brand>;
            let mut guard = DropGuard {
                ptr: buf_ptr,
                initialized: 0,
            };

            for (i, node) in nodes.iter().enumerate() {
                unsafe {
                    buf_ptr.add(i).write(NumaNodePlacement {
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
                    split.push(NumaNodePlacement {
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
        ConstNumaNodePlacement<'brand, A>,
        ConstNumaNodePlacement<'brand, B>,
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
                ConstNumaNodePlacement {
                    token: core::ptr::read(&this.token),
                },
                ConstNumaNodePlacement {
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
    ) -> ConstNumaNodePlacement<'brand, NODE_ID> {
        let this = core::mem::ManuallyDrop::new(self);
        ConstNumaNodePlacement {
            token: unsafe { core::ptr::read(&this.token) },
        }
    }
}

/// A trait for cells pinned to a specific NUMA node.
pub trait PinnedCell<'brand, T> {
    /// Returns the pinned NUMA node ID.
    fn node_id(&self) -> crate::law::NumaNodeId;

    /// Access the underlying MelinoeCell.
    fn cell(&self) -> &MelinoeCell<'brand, T>;
}

/// A trait for cells pinned statically to a specific NUMA node.
pub trait ConstPinnedCell<'brand, const NODE_ID: u32, T> {
    /// Access the underlying MelinoeCell.
    fn cell(&self) -> &MelinoeCell<'brand, T>;
}

/// A trait for contiguous slices of cells pinned to a specific NUMA node.
pub trait PinnedSlice<'brand, T> {
    /// Returns the pinned NUMA node ID.
    fn node_id(&self) -> crate::law::NumaNodeId;

    /// Access the underlying slice of MelinoeCell.
    fn cells(&self) -> &[MelinoeCell<'brand, T>];
}

/// A trait for contiguous slices of cells pinned statically to a specific NUMA node.
pub trait ConstPinnedSlice<'brand, const NODE_ID: u32, T> {
    /// Access the underlying slice of MelinoeCell.
    fn cells(&self) -> &[MelinoeCell<'brand, T>];
}

/// A placement cell pinned to a specific NUMA node.
pub struct NumaPinnedCell<'brand, T> {
    node_id: crate::law::NumaNodeId,
    cell: MelinoeCell<'brand, T>,
}

impl<'brand, T> NumaPinnedCell<'brand, T> {
    /// Creates a new cell pinned to the specified NUMA node.
    #[must_use]
    pub const fn new(node_id: crate::law::NumaNodeId, value: T) -> Self {
        Self {
            node_id,
            cell: MelinoeCell::new(value),
        }
    }

    /// Returns the pinned NUMA node ID.
    #[must_use]
    pub const fn node_id(&self) -> crate::law::NumaNodeId {
        self.node_id
    }
}

impl<'brand, T> PinnedCell<'brand, T> for NumaPinnedCell<'brand, T> {
    #[inline]
    fn node_id(&self) -> crate::law::NumaNodeId {
        self.node_id
    }

    #[inline]
    fn cell(&self) -> &MelinoeCell<'brand, T> {
        &self.cell
    }
}

/// A borrowed reference to a cell pinned to a specific NUMA node.
pub struct NumaPinnedCellRef<'a, 'brand, T> {
    node_id: crate::law::NumaNodeId,
    cell: &'a MelinoeCell<'brand, T>,
}

impl<'a, 'brand, T> NumaPinnedCellRef<'a, 'brand, T> {
    /// Creates a new pinned cell reference.
    #[must_use]
    #[inline]
    pub const fn new(node_id: crate::law::NumaNodeId, cell: &'a MelinoeCell<'brand, T>) -> Self {
        Self { node_id, cell }
    }
}

impl<'a, 'brand, T> PinnedCell<'brand, T> for NumaPinnedCellRef<'a, 'brand, T> {
    #[inline]
    fn node_id(&self) -> crate::law::NumaNodeId {
        self.node_id
    }

    #[inline]
    fn cell(&self) -> &MelinoeCell<'brand, T> {
        self.cell
    }
}

/// A contiguous slice of cells pinned to a specific NUMA node.
pub struct NumaPinnedSlice<'brand, T> {
    node_id: crate::law::NumaNodeId,
    cells: Box<[MelinoeCell<'brand, T>]>,
}

impl<'brand, T> NumaPinnedSlice<'brand, T> {
    /// Creates a new pinned slice from a vector of values.
    #[must_use]
    pub fn new(node_id: crate::law::NumaNodeId, values: Vec<T>) -> Self {
        let cells = values
            .into_iter()
            .map(MelinoeCell::new)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self { node_id, cells }
    }

    /// Creates a new pinned slice directly from a boxed slice of cells.
    #[must_use]
    pub const fn from_cells(node_id: crate::law::NumaNodeId, cells: Box<[MelinoeCell<'brand, T>]>) -> Self {
        Self { node_id, cells }
    }

    /// Returns the pinned NUMA node ID.
    #[must_use]
    pub const fn node_id(&self) -> crate::law::NumaNodeId {
        self.node_id
    }

    /// Borrow the underlying cells immutably.
    #[must_use]
    #[inline]
    pub fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        &self.cells
    }
}

impl<'brand, T> PinnedSlice<'brand, T> for NumaPinnedSlice<'brand, T> {
    #[inline]
    fn node_id(&self) -> crate::law::NumaNodeId {
        self.node_id
    }

    #[inline]
    fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        &self.cells
    }
}

/// A borrowed reference to a contiguous slice of cells pinned to a specific NUMA node.
pub struct NumaPinnedSliceRef<'a, 'brand, T> {
    node_id: crate::law::NumaNodeId,
    cells: &'a [MelinoeCell<'brand, T>],
}

impl<'a, 'brand, T> NumaPinnedSliceRef<'a, 'brand, T> {
    /// Creates a new pinned slice reference.
    #[must_use]
    #[inline]
    pub const fn new(node_id: crate::law::NumaNodeId, cells: &'a [MelinoeCell<'brand, T>]) -> Self {
        Self { node_id, cells }
    }
}

impl<'a, 'brand, T> PinnedSlice<'brand, T> for NumaPinnedSliceRef<'a, 'brand, T> {
    #[inline]
    fn node_id(&self) -> crate::law::NumaNodeId {
        self.node_id
    }

    #[inline]
    fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        self.cells
    }
}

/// A node-specific placement capability.
pub struct NumaNodePlacement<'brand> {
    node_id: crate::law::NumaNodeId,
    token: SyncRegionToken<'brand>,
}

impl<'brand> NumaNodePlacement<'brand> {
    /// Returns the assigned NUMA node ID.
    #[must_use]
    pub const fn node_id(&self) -> crate::law::NumaNodeId {
        self.node_id
    }

    /// Reads state from a cell pinned to the same NUMA node.
    #[inline]
    pub fn read<'a, C, T>(&'a self, cell: &'a C) -> Option<MelinoeRef<'a, 'brand, T>>
    where
        C: PinnedCell<'brand, T> + ?Sized,
    {
        if self.node_id == cell.node_id() {
            Some(cell.cell().borrow(&self.token))
        } else {
            None
        }
    }

    /// Writes state to a cell pinned to the same NUMA node.
    #[inline]
    pub fn write<'a, C, T>(
        &'a mut self,
        cell: &'a C,
    ) -> Option<MelinoeMut<'a, 'brand, T>>
    where
        C: PinnedCell<'brand, T> + ?Sized,
    {
        if self.node_id == cell.node_id() {
            Some(cell.cell().borrow_mut(&mut self.token))
        } else {
            None
        }
    }

    /// Reads a slice pinned to the same NUMA node.
    #[inline]
    pub fn read_slice<'a, S, T>(
        &'a self,
        slice: &'a S,
    ) -> Option<&'a [T]>
    where
        S: PinnedSlice<'brand, T> + ?Sized,
    {
        if self.node_id == slice.node_id() {
            use melinoe::CellSliceExt;
            Some(slice.cells().borrow_slice(&self.token))
        } else {
            None
        }
    }

    /// Writes to a slice pinned to the same NUMA node.
    #[inline]
    pub fn write_slice<'a, S, T>(
        &'a mut self,
        slice: &'a S,
    ) -> Option<&'a mut [T]>
    where
        S: PinnedSlice<'brand, T> + ?Sized,
    {
        if self.node_id == slice.node_id() {
            use melinoe::CellSliceExt;
            Some(slice.cells().borrow_slice_mut(&mut self.token))
        } else {
            None
        }
    }
}

/// A placement cell pinned to a specific NUMA node, verified at compile time.
pub struct ConstNumaPinnedCell<'brand, const NODE_ID: u32, T> {
    cell: MelinoeCell<'brand, T>,
}

impl<'brand, const NODE_ID: u32, T> ConstNumaPinnedCell<'brand, NODE_ID, T> {
    /// Creates a new cell pinned statically to the node ID.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self {
            cell: MelinoeCell::new(value),
        }
    }
}

impl<'brand, const NODE_ID: u32, T> ConstPinnedCell<'brand, NODE_ID, T> for ConstNumaPinnedCell<'brand, NODE_ID, T> {
    #[inline]
    fn cell(&self) -> &MelinoeCell<'brand, T> {
        &self.cell
    }
}

/// A borrowed reference to a cell pinned statically to a specific NUMA node.
pub struct ConstNumaPinnedCellRef<'a, 'brand, const NODE_ID: u32, T> {
    cell: &'a MelinoeCell<'brand, T>,
}

impl<'a, 'brand, const NODE_ID: u32, T> ConstNumaPinnedCellRef<'a, 'brand, NODE_ID, T> {
    /// Creates a new statically pinned cell reference.
    #[must_use]
    #[inline]
    pub const fn new(cell: &'a MelinoeCell<'brand, T>) -> Self {
        Self { cell }
    }
}

impl<'a, 'brand, const NODE_ID: u32, T> ConstPinnedCell<'brand, NODE_ID, T> for ConstNumaPinnedCellRef<'a, 'brand, NODE_ID, T> {
    #[inline]
    fn cell(&self) -> &MelinoeCell<'brand, T> {
        self.cell
    }
}

/// A contiguous slice of cells pinned to a specific NUMA node, verified at compile time.
pub struct ConstNumaPinnedSlice<'brand, const NODE_ID: u32, T> {
    cells: Box<[MelinoeCell<'brand, T>]>,
}

impl<'brand, const NODE_ID: u32, T> ConstNumaPinnedSlice<'brand, NODE_ID, T> {
    /// Creates a new pinned slice from a vector of values.
    #[must_use]
    pub fn new(values: Vec<T>) -> Self {
        let cells = values
            .into_iter()
            .map(MelinoeCell::new)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self { cells }
    }

    /// Creates a new pinned slice directly from a boxed slice of cells.
    #[must_use]
    pub const fn from_cells(cells: Box<[MelinoeCell<'brand, T>]>) -> Self {
        Self { cells }
    }

    /// Borrow the underlying cells immutably.
    #[must_use]
    #[inline]
    pub fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        &self.cells
    }
}

impl<'brand, const NODE_ID: u32, T> ConstPinnedSlice<'brand, NODE_ID, T> for ConstNumaPinnedSlice<'brand, NODE_ID, T> {
    #[inline]
    fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        &self.cells
    }
}

/// A borrowed reference to a contiguous slice of cells pinned statically to a specific NUMA node.
pub struct ConstNumaPinnedSliceRef<'a, 'brand, const NODE_ID: u32, T> {
    cells: &'a [MelinoeCell<'brand, T>],
}

impl<'a, 'brand, const NODE_ID: u32, T> ConstNumaPinnedSliceRef<'a, 'brand, NODE_ID, T> {
    /// Creates a new statically pinned slice reference.
    #[must_use]
    #[inline]
    pub const fn new(cells: &'a [MelinoeCell<'brand, T>]) -> Self {
        Self { cells }
    }
}

impl<'a, 'brand, const NODE_ID: u32, T> ConstPinnedSlice<'brand, NODE_ID, T> for ConstNumaPinnedSliceRef<'a, 'brand, NODE_ID, T> {
    #[inline]
    fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        self.cells
    }
}

/// A node-specific placement capability, verified at compile time.
pub struct ConstNumaNodePlacement<'brand, const NODE_ID: u32> {
    token: SyncRegionToken<'brand>,
}

impl<'brand, const NODE_ID: u32> ConstNumaNodePlacement<'brand, NODE_ID> {
    /// Returns the assigned NUMA node ID.
    #[must_use]
    pub const fn node_id(&self) -> u32 {
        NODE_ID
    }

    /// Reads state from a cell pinned statically to the same NUMA node.
    #[inline]
    pub fn read<'a, C, T>(
        &'a self,
        cell: &'a C,
    ) -> MelinoeRef<'a, 'brand, T>
    where
        C: ConstPinnedCell<'brand, NODE_ID, T> + ?Sized,
    {
        cell.cell().borrow(&self.token)
    }

    /// Writes state to a cell pinned statically to the same NUMA node.
    #[inline]
    pub fn write<'a, C, T>(
        &'a mut self,
        cell: &'a C,
    ) -> MelinoeMut<'a, 'brand, T>
    where
        C: ConstPinnedCell<'brand, NODE_ID, T> + ?Sized,
    {
        cell.cell().borrow_mut(&mut self.token)
    }

    /// Reads a slice pinned statically to the same NUMA node.
    #[inline]
    pub fn read_slice<'a, S, T>(
        &'a self,
        slice: &'a S,
    ) -> &'a [T]
    where
        S: ConstPinnedSlice<'brand, NODE_ID, T> + ?Sized,
    {
        use melinoe::CellSliceExt;
        slice.cells().borrow_slice(&self.token)
    }

    /// Writes to a slice pinned statically to the same NUMA node.
    #[inline]
    pub fn write_slice<'a, S, T>(
        &'a mut self,
        slice: &'a S,
    ) -> &'a mut [T]
    where
        S: ConstPinnedSlice<'brand, NODE_ID, T> + ?Sized,
    {
        use melinoe::CellSliceExt;
        slice.cells().borrow_slice_mut(&mut self.token)
    }
}

/// Opens a thread-portable placement-region scope.
#[inline]
pub fn sync_region_placement_scope<R>(
    f: impl for<'brand> FnOnce(SyncRegionPlacement<'brand>) -> R,
) -> R {
    sync_region_scope(|token| f(SyncRegionPlacement { token }))
}
