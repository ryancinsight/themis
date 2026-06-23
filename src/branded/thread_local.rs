//! Thread-confined branded placement scopes.

use melinoe::sync::{thread_local_scope, ThreadLocalToken};
use melinoe::{MelinoeCell, MelinoeMut, MelinoeRef};

/// Thread-confined placement capability.
///
/// The inner Melinoe token is `!Send + !Sync`, so placement state accessed
/// through this scope cannot cross thread boundaries.
pub struct ThreadLocalPlacement<'brand> {
    token: ThreadLocalToken<'brand>,
}

impl<'brand> ThreadLocalPlacement<'brand> {
    /// Creates a Melinoe cell in this placement brand.
    #[must_use]
    #[inline]
    pub const fn cell<T>(&self, value: T) -> MelinoeCell<'brand, T> {
        MelinoeCell::new(value)
    }

    /// Reads placement state through the thread-confined permit.
    #[inline]
    pub fn read<'a, T>(&'a self, cell: &'a MelinoeCell<'brand, T>) -> MelinoeRef<'a, 'brand, T> {
        cell.borrow(&self.token)
    }

    /// Writes placement state through the thread-confined permit.
    #[inline]
    pub fn write<'a, T>(
        &'a mut self,
        cell: &'a MelinoeCell<'brand, T>,
    ) -> MelinoeMut<'a, 'brand, T> {
        cell.borrow_mut(&mut self.token)
    }

    /// Locks this thread-confined capability to the current NUMA node.
    #[must_use]
    #[inline]
    pub fn pin_local(self) -> ThreadLocalNumaPlacement<'brand> {
        ThreadLocalNumaPlacement {
            node_id: crate::query::current_numa_node(),
            token: self.token,
        }
    }

    /// Locks this thread-confined capability to a static NUMA node.
    #[must_use]
    #[inline]
    pub fn pin_local_static<const NODE_ID: u32>(
        self,
    ) -> ConstThreadLocalNumaPlacement<'brand, NODE_ID> {
        ConstThreadLocalNumaPlacement { token: self.token }
    }
}

/// A node-specific thread-confined placement capability.
pub struct ThreadLocalNumaPlacement<'brand> {
    node_id: crate::law::NumaNodeId,
    token: melinoe::sync::ThreadLocalToken<'brand>,
}

impl<'brand> ThreadLocalNumaPlacement<'brand> {
    /// Returns the assigned NUMA node ID.
    #[must_use]
    pub const fn node_id(&self) -> crate::law::NumaNodeId {
        self.node_id
    }

    /// Reads state from a cell pinned to the same NUMA node.
    #[inline]
    pub fn read<'a, C, T>(
        &'a self,
        cell: &'a C,
    ) -> Option<MelinoeRef<'a, 'brand, T>>
    where
        C: crate::branded::PinnedCell<'brand, T> + ?Sized,
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
        C: crate::branded::PinnedCell<'brand, T> + ?Sized,
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
        S: crate::branded::PinnedSlice<'brand, T> + ?Sized,
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
        S: crate::branded::PinnedSlice<'brand, T> + ?Sized,
    {
        if self.node_id == slice.node_id() {
            use melinoe::CellSliceExt;
            Some(slice.cells().borrow_slice_mut(&mut self.token))
        } else {
            None
        }
    }
}

/// A node-specific thread-confined placement capability, verified at compile time.
pub struct ConstThreadLocalNumaPlacement<'brand, const NODE_ID: u32> {
    token: ThreadLocalToken<'brand>,
}

impl<'brand, const NODE_ID: u32> ConstThreadLocalNumaPlacement<'brand, NODE_ID> {
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
        C: crate::branded::ConstPinnedCell<'brand, NODE_ID, T> + ?Sized,
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
        C: crate::branded::ConstPinnedCell<'brand, NODE_ID, T> + ?Sized,
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
        S: crate::branded::ConstPinnedSlice<'brand, NODE_ID, T> + ?Sized,
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
        S: crate::branded::ConstPinnedSlice<'brand, NODE_ID, T> + ?Sized,
    {
        use melinoe::CellSliceExt;
        slice.cells().borrow_slice_mut(&mut self.token)
    }
}

/// Opens a thread-confined placement scope.
#[inline]
pub fn thread_local_placement_scope<R>(
    f: impl for<'brand> FnOnce(ThreadLocalPlacement<'brand>) -> R,
) -> R {
    thread_local_scope(|token| f(ThreadLocalPlacement { token }))
}
