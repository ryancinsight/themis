use melinoe::sync::SyncRegionToken;
use melinoe::{CellSliceExt, MelinoeMut, MelinoeRef};

use super::cell::{ConstPinnedCell, ConstPinnedSlice, PinnedCell, PinnedSlice};

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
}
