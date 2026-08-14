//! Statically tagged branded placement cells.

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};

use melinoe::{collections::BrandedVec, MelinoeCell};

use super::cell::{ConstPinnedCell, ConstPinnedSlice};

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

    /// Borrows this cell as a reference carrying the same static pin.
    ///
    /// `NODE_ID` is inherited from the owner rather than chosen at the call
    /// site, so the reference cannot relabel the cell onto another NUMA node.
    #[must_use]
    #[inline]
    pub const fn as_pinned_ref(&self) -> ConstNumaPinnedCellRef<'_, 'brand, NODE_ID, T> {
        ConstNumaPinnedCellRef { cell: &self.cell }
    }
}

// SAFETY: the cell is owned by this struct and created by its constructor, so
// no other pinned wrapper can name it. `NODE_ID` is part of the type, so the
// tag cannot vary between calls.
unsafe impl<'brand, const NODE_ID: u32, T> ConstPinnedCell<'brand, NODE_ID, T>
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
    /// Pins an exclusively borrowed cell to `NODE_ID`.
    ///
    /// The `&mut` borrow is the placement proof: it is consumed for `'a`, so
    /// the compiler rejects a second reference to `cell` and therefore a second
    /// `NODE_ID` for it. Two capabilities minted by
    /// [`split_static`](crate::SyncRegionPlacement::split_static) can never
    /// both reach one cell.
    ///
    /// ```
    /// use themis::{ConstNumaPinnedCellRef, sync_region_placement_scope};
    ///
    /// sync_region_placement_scope(|region| {
    ///     let mut cell = region.cell(7u32);
    ///     let pinned = ConstNumaPinnedCellRef::<0, _>::from_unique(&mut cell);
    ///     let mut placement = region.project_static::<0>();
    ///     assert_eq!(*placement.read(&pinned), 7);
    /// });
    /// ```
    #[must_use]
    #[inline]
    pub const fn from_unique(cell: &'a mut MelinoeCell<'brand, T>) -> Self {
        Self { cell }
    }
}

// SAFETY: the borrowed cell reached this wrapper either through
// `from_unique`, which consumes an exclusive borrow for `'a` and so precludes a
// second wrapper over the same cell, or through
// `ConstNumaPinnedCell::as_pinned_ref`, which inherits the owner's `NODE_ID`.
// Either way the cell answers to exactly this tag, and `NODE_ID` being part of
// the type keeps it constant.
unsafe impl<'brand, const NODE_ID: u32, T> ConstPinnedCell<'brand, NODE_ID, T>
    for ConstNumaPinnedCellRef<'_, 'brand, NODE_ID, T>
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

    /// Borrows these cells as a slice reference carrying the same static pin.
    ///
    /// `NODE_ID` is inherited from the owner rather than chosen at the call
    /// site.
    #[must_use]
    #[inline]
    pub const fn as_pinned_ref(&self) -> ConstNumaPinnedSliceRef<'_, 'brand, NODE_ID, T> {
        ConstNumaPinnedSliceRef { cells: &self.cells }
    }
}

// SAFETY: the cells are owned by this struct and produced by its constructors,
// so no other pinned wrapper can name them. `NODE_ID` is part of the type, so
// the tag cannot vary between calls.
unsafe impl<'brand, const NODE_ID: u32, T> ConstPinnedSlice<'brand, NODE_ID, T>
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
    /// Pins an exclusively borrowed cell slice to `NODE_ID`.
    ///
    /// The `&mut` borrow is the placement proof: it is consumed for `'a`, so no
    /// second wrapper — and no second `NODE_ID` — can cover these cells while
    /// this one lives. Placing a stack array needs no allocation.
    #[must_use]
    #[inline]
    pub const fn from_unique(cells: &'a mut [MelinoeCell<'brand, T>]) -> Self {
        Self { cells }
    }
}

// SAFETY: the borrowed cells reached this wrapper either through
// `from_unique`, which consumes an exclusive borrow for `'a` and so precludes a
// second wrapper over the same cells, or through
// `ConstNumaPinnedSlice::as_pinned_ref`, which inherits the owner's `NODE_ID`.
// Either way every cell answers to exactly this tag.
unsafe impl<'brand, const NODE_ID: u32, T> ConstPinnedSlice<'brand, NODE_ID, T>
    for ConstNumaPinnedSliceRef<'_, 'brand, NODE_ID, T>
{
    #[inline]
    fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        self.cells
    }
}
