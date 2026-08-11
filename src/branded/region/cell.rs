#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};

use melinoe::{collections::BrandedVec, MelinoeCell};

use crate::NumaNodeId;

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// A trait for cells pinned to a specific NUMA node.
pub trait PinnedCell<'brand, T> {
    /// Returns the pinned NUMA node ID.
    fn node_id(&self) -> NumaNodeId;

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
    fn node_id(&self) -> NumaNodeId;

    /// Access the underlying slice of MelinoeCell.
    fn cells(&self) -> &[MelinoeCell<'brand, T>];
}

/// A trait for contiguous slices of cells pinned statically to a specific NUMA node.
pub trait ConstPinnedSlice<'brand, const NODE_ID: u32, T> {
    /// Access the underlying slice of MelinoeCell.
    fn cells(&self) -> &[MelinoeCell<'brand, T>];
}

// ---------------------------------------------------------------------------
// Dynamic pinned types
// ---------------------------------------------------------------------------

/// A placement cell pinned to a specific NUMA node.
pub struct NumaPinnedCell<'brand, T> {
    node_id: NumaNodeId,
    cell: MelinoeCell<'brand, T>,
}

impl<'brand, T> NumaPinnedCell<'brand, T> {
    /// Creates a new cell pinned to the specified NUMA node.
    #[must_use]
    pub const fn new(node_id: NumaNodeId, value: T) -> Self {
        Self {
            node_id,
            cell: MelinoeCell::new(value),
        }
    }

    /// Returns the pinned NUMA node ID.
    #[must_use]
    pub const fn node_id(&self) -> NumaNodeId {
        self.node_id
    }
}

impl<'brand, T> PinnedCell<'brand, T> for NumaPinnedCell<'brand, T> {
    #[inline]
    fn node_id(&self) -> NumaNodeId {
        self.node_id
    }

    #[inline]
    fn cell(&self) -> &MelinoeCell<'brand, T> {
        &self.cell
    }
}

/// A borrowed reference to a cell pinned to a specific NUMA node.
pub struct NumaPinnedCellRef<'a, 'brand, T> {
    node_id: NumaNodeId,
    cell: &'a MelinoeCell<'brand, T>,
}

impl<'a, 'brand, T> NumaPinnedCellRef<'a, 'brand, T> {
    /// Creates a new pinned cell reference.
    #[must_use]
    #[inline]
    pub const fn new(node_id: NumaNodeId, cell: &'a MelinoeCell<'brand, T>) -> Self {
        Self { node_id, cell }
    }
}

impl<'a, 'brand, T> PinnedCell<'brand, T> for NumaPinnedCellRef<'a, 'brand, T> {
    #[inline]
    fn node_id(&self) -> NumaNodeId {
        self.node_id
    }

    #[inline]
    fn cell(&self) -> &MelinoeCell<'brand, T> {
        self.cell
    }
}

/// A contiguous slice of cells pinned to a specific NUMA node.
pub struct NumaPinnedSlice<'brand, T> {
    node_id: NumaNodeId,
    cells: Box<[MelinoeCell<'brand, T>]>,
}

impl<'brand, T> NumaPinnedSlice<'brand, T> {
    /// Creates a new pinned slice from a vector of values.
    #[must_use]
    pub fn new(node_id: NumaNodeId, values: Vec<T>) -> Self {
        let cells = BrandedVec::from_iter(values).into_boxed_cells();
        Self { node_id, cells }
    }

    /// Creates a new pinned slice by generating values in index order.
    ///
    /// Generation is performed directly through Melinoe's branded collection
    /// primitive; no intermediate unbranded value vector is required.
    #[must_use]
    pub fn from_fn<F>(node_id: NumaNodeId, len: usize, generate: F) -> Self
    where
        F: FnMut(usize) -> T,
    {
        let cells = BrandedVec::from_fn(len, generate).into_boxed_cells();
        Self { node_id, cells }
    }

    /// Creates a new pinned slice directly from a boxed slice of cells.
    #[must_use]
    pub const fn from_cells(node_id: NumaNodeId, cells: Box<[MelinoeCell<'brand, T>]>) -> Self {
        Self { node_id, cells }
    }

    /// Returns the pinned NUMA node ID.
    #[must_use]
    pub const fn node_id(&self) -> NumaNodeId {
        self.node_id
    }

    /// Borrow the underlying cells immutably.
    #[must_use]
    #[inline]
    pub fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        &self.cells
    }

    /// Access the uniquely owned branded cells for Themis's placement-gated
    /// Melinoe partition driver.
    #[cfg(feature = "std")]
    pub(crate) fn cells_mut(&mut self) -> &mut [MelinoeCell<'brand, T>] {
        &mut self.cells
    }
}

impl<'brand, T> PinnedSlice<'brand, T> for NumaPinnedSlice<'brand, T> {
    #[inline]
    fn node_id(&self) -> NumaNodeId {
        self.node_id
    }

    #[inline]
    fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        &self.cells
    }
}

/// A borrowed reference to a contiguous slice of cells pinned to a specific NUMA node.
pub struct NumaPinnedSliceRef<'a, 'brand, T> {
    node_id: NumaNodeId,
    cells: &'a [MelinoeCell<'brand, T>],
}

impl<'a, 'brand, T> NumaPinnedSliceRef<'a, 'brand, T> {
    /// Creates a new pinned slice reference.
    #[must_use]
    #[inline]
    pub const fn new(node_id: NumaNodeId, cells: &'a [MelinoeCell<'brand, T>]) -> Self {
        Self { node_id, cells }
    }
}

impl<'a, 'brand, T> PinnedSlice<'brand, T> for NumaPinnedSliceRef<'a, 'brand, T> {
    #[inline]
    fn node_id(&self) -> NumaNodeId {
        self.node_id
    }

    #[inline]
    fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        self.cells
    }
}

// ---------------------------------------------------------------------------
// Const generic pinned types
// ---------------------------------------------------------------------------

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

impl<'brand, const NODE_ID: u32, T> ConstPinnedCell<'brand, NODE_ID, T>
    for ConstNumaPinnedCell<'brand, NODE_ID, T>
{
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

impl<'a, 'brand, const NODE_ID: u32, T> ConstPinnedCell<'brand, NODE_ID, T>
    for ConstNumaPinnedCellRef<'a, 'brand, NODE_ID, T>
{
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
        let cells = BrandedVec::from_iter(values).into_boxed_cells();
        Self { cells }
    }

    /// Creates a new pinned slice by generating values in index order.
    ///
    /// Generation is performed directly through Melinoe's branded collection
    /// primitive; no intermediate unbranded value vector is required.
    #[must_use]
    pub fn from_fn<F>(len: usize, generate: F) -> Self
    where
        F: FnMut(usize) -> T,
    {
        let cells = BrandedVec::from_fn(len, generate).into_boxed_cells();
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

    /// Access the uniquely owned branded cells for Themis's placement-gated
    /// Melinoe partition driver.
    #[cfg(feature = "std")]
    pub(crate) fn cells_mut(&mut self) -> &mut [MelinoeCell<'brand, T>] {
        &mut self.cells
    }
}

impl<'brand, const NODE_ID: u32, T> ConstPinnedSlice<'brand, NODE_ID, T>
    for ConstNumaPinnedSlice<'brand, NODE_ID, T>
{
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

impl<'a, 'brand, const NODE_ID: u32, T> ConstPinnedSlice<'brand, NODE_ID, T>
    for ConstNumaPinnedSliceRef<'a, 'brand, NODE_ID, T>
{
    #[inline]
    fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        self.cells
    }
}
