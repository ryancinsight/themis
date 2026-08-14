use melinoe::sync::SyncRegionToken;
use melinoe::{CellSliceExt, MelinoeMut, MelinoeRef};

use super::cell::{ConstPinnedCell, ConstPinnedSlice, NumaPinnedSlice, PinnedCell, PinnedSlice};
use super::static_cell::ConstNumaPinnedSlice;

/// A node-specific placement capability.
pub struct NumaNodePlacement<'brand> {
    pub(super) node_id: crate::NumaNodeId,
    pub(super) token: SyncRegionToken<'brand>,
}

impl<'brand> NumaNodePlacement<'brand> {
    /// Returns the assigned NUMA node ID.
    #[must_use]
    pub const fn node_id(&self) -> crate::NumaNodeId {
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
    pub fn write<'a, C, T>(&'a mut self, cell: &'a C) -> Option<MelinoeMut<'a, 'brand, T>>
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
    pub fn read_slice<'a, S, T>(&'a self, slice: &'a S) -> Option<&'a [T]>
    where
        S: PinnedSlice<'brand, T> + ?Sized,
    {
        if self.node_id == slice.node_id() {
            Some(slice.cells().borrow_slice(&self.token))
        } else {
            None
        }
    }

    /// Writes to a slice pinned to the same NUMA node.
    #[inline]
    pub fn write_slice<'a, S, T>(&'a mut self, slice: &'a S) -> Option<&'a mut [T]>
    where
        S: PinnedSlice<'brand, T> + ?Sized,
    {
        if self.node_id == slice.node_id() {
            Some(slice.cells().borrow_slice_mut(&mut self.token))
        } else {
            None
        }
    }

    /// Mutate disjoint portions of a matching, uniquely owned pinned slice
    /// through Melinoe's parallel partition driver.
    ///
    /// The mutable slice borrow establishes unique ownership; this permit adds
    /// the dynamic NUMA-node validation before Melinoe executes the shards.
    #[cfg(feature = "std")]
    pub fn partition_for_each_mut_with<T, F>(
        &mut self,
        slice: &mut NumaPinnedSlice<'brand, T>,
        plan: melinoe::sync::PartitionPlan,
        f: F,
    ) -> Option<()>
    where
        T: Send,
        F: Fn(usize, &mut [T]) + Sync,
    {
        if self.node_id != slice.node_id() {
            return None;
        }
        melinoe::sync::partition_for_each_with(slice.cells_mut(), plan, |start, mut shard| {
            f(start, shard.as_mut_slice());
        });
        Some(())
    }
}

/// A node-specific placement capability, verified at compile time.
pub struct ConstNumaNodePlacement<'brand, const NODE_ID: u32> {
    pub(super) token: SyncRegionToken<'brand>,
}

impl<'brand, const NODE_ID: u32> ConstNumaNodePlacement<'brand, NODE_ID> {
    /// Returns the assigned NUMA node ID.
    #[must_use]
    pub const fn node_id(&self) -> u32 {
        NODE_ID
    }

    /// Reads state from a cell pinned statically to the same NUMA node.
    #[inline]
    pub fn read<'a, C, T>(&'a self, cell: &'a C) -> MelinoeRef<'a, 'brand, T>
    where
        C: ConstPinnedCell<'brand, NODE_ID, T> + ?Sized,
    {
        cell.cell().borrow(&self.token)
    }

    /// Writes state to a cell pinned statically to the same NUMA node.
    #[inline]
    pub fn write<'a, C, T>(&'a mut self, cell: &'a C) -> MelinoeMut<'a, 'brand, T>
    where
        C: ConstPinnedCell<'brand, NODE_ID, T> + ?Sized,
    {
        cell.cell().borrow_mut(&mut self.token)
    }

    /// Reads a slice pinned statically to the same NUMA node.
    #[inline]
    pub fn read_slice<'a, S, T>(&'a self, slice: &'a S) -> &'a [T]
    where
        S: ConstPinnedSlice<'brand, NODE_ID, T> + ?Sized,
    {
        slice.cells().borrow_slice(&self.token)
    }

    /// Writes to a slice pinned statically to the same NUMA node.
    #[inline]
    pub fn write_slice<'a, S, T>(&'a mut self, slice: &'a S) -> &'a mut [T]
    where
        S: ConstPinnedSlice<'brand, NODE_ID, T> + ?Sized,
    {
        slice.cells().borrow_slice_mut(&mut self.token)
    }

    /// Mutate disjoint portions of a statically matching, uniquely owned
    /// pinned slice through Melinoe's parallel partition driver.
    ///
    /// The mutable slice borrow establishes unique ownership; the const-generic
    /// permit supplies the compile-time NUMA placement identity.
    #[cfg(feature = "std")]
    pub fn partition_for_each_mut_with<T, F>(
        &mut self,
        slice: &mut ConstNumaPinnedSlice<'brand, NODE_ID, T>,
        plan: melinoe::sync::PartitionPlan,
        f: F,
    ) where
        T: Send,
        F: Fn(usize, &mut [T]) + Sync,
    {
        melinoe::sync::partition_for_each_with(slice.cells_mut(), plan, |start, mut shard| {
            f(start, shard.as_mut_slice());
        });
    }
}
