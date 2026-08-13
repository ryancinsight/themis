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
//
// # The placement-partition contract
//
// Melinoe grants one write token per brand, and that single token is what
// makes `&mut T` through a `MelinoeCell` unaliased. `SyncRegionPlacement`'s
// split operations hand out several capabilities per brand, so they duplicate
// that token: each per-node capability owns a copy. The token can no longer
// carry the exclusion, so the *node tag* has to.
//
// That works only if the tag genuinely partitions the cells — if every cell is
// reachable through exactly one tag. Ownership supplies that proof (an owned
// pinned cell mints its own `MelinoeCell` and no other wrapper can name it), as
// does an exclusive borrow held for the wrapper's whole life. A tag attached to
// a shared `&MelinoeCell` supplies nothing: the same cell can then be labelled
// twice and two capabilities will each hand out `&mut T` for it.
//
// The four traits below are the dispatch surface the placement `write` methods
// use, so they are the point where that proof must be demanded. They are
// `unsafe` traits for that reason.

/// A cell pinned to a specific NUMA node.
///
/// # Safety
///
/// The [`MelinoeCell`] returned by [`cell`](PinnedCell::cell) must not be
/// reachable, for as long as `self` lives, through any other [`PinnedCell`],
/// [`ConstPinnedCell`], [`PinnedSlice`], or [`ConstPinnedSlice`] value whose
/// node tag differs from this one's [`node_id`](PinnedCell::node_id).
/// Coexisting placement capabilities are separated by tag alone, so a cell
/// answering to two tags yields two live `&mut T` to one location.
///
/// Owning the cell discharges the obligation; so does holding an exclusive
/// borrow of it for `self`'s lifetime. Wrapping a shared `&MelinoeCell`
/// alongside a caller-chosen node id does not.
///
/// [`node_id`](PinnedCell::node_id) must also be pure — a tag that varies
/// between calls lets one cell answer to two capabilities.
pub unsafe trait PinnedCell<'brand, T> {
    /// Returns the pinned NUMA node ID.
    fn node_id(&self) -> NumaNodeId;

    /// Access the underlying [`MelinoeCell`].
    fn cell(&self) -> &MelinoeCell<'brand, T>;
}

/// A cell pinned statically to a specific NUMA node.
///
/// # Safety
///
/// As [`PinnedCell`], with `NODE_ID` as the tag: the [`MelinoeCell`] returned
/// by [`cell`](ConstPinnedCell::cell) must be unreachable through any pinned
/// wrapper carrying a different node tag for as long as `self` lives.
pub unsafe trait ConstPinnedCell<'brand, const NODE_ID: u32, T> {
    /// Access the underlying [`MelinoeCell`].
    fn cell(&self) -> &MelinoeCell<'brand, T>;
}

/// A contiguous slice of cells pinned to a specific NUMA node.
///
/// # Safety
///
/// As [`PinnedCell`], applied elementwise: no cell in the slice returned by
/// [`cells`](PinnedSlice::cells) may be reachable through a pinned wrapper
/// carrying a different node tag for as long as `self` lives.
pub unsafe trait PinnedSlice<'brand, T> {
    /// Returns the pinned NUMA node ID.
    fn node_id(&self) -> NumaNodeId;

    /// Access the underlying slice of [`MelinoeCell`].
    fn cells(&self) -> &[MelinoeCell<'brand, T>];
}

/// A contiguous slice of cells pinned statically to a specific NUMA node.
///
/// # Safety
///
/// As [`PinnedSlice`], with `NODE_ID` as the tag.
pub unsafe trait ConstPinnedSlice<'brand, const NODE_ID: u32, T> {
    /// Access the underlying slice of [`MelinoeCell`].
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

    /// Borrows this cell as a reference carrying the same pin.
    ///
    /// The tag is inherited from the owner rather than supplied by the caller,
    /// so the reference cannot relabel the cell onto another NUMA node.
    #[must_use]
    #[inline]
    pub const fn as_pinned_ref(&self) -> NumaPinnedCellRef<'_, 'brand, T> {
        NumaPinnedCellRef {
            node_id: self.node_id,
            cell: &self.cell,
        }
    }
}

// SAFETY: the cell is owned by this struct and created by its constructor, so
// no other pinned wrapper can name it; `node_id` returns a `Copy` field that is
// fixed at construction.
unsafe impl<'brand, T> PinnedCell<'brand, T> for NumaPinnedCell<'brand, T> {
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
    /// Pins an exclusively borrowed cell to `node_id`.
    ///
    /// The `&mut` borrow is the placement proof and the reason this is safe:
    /// it is consumed for `'a`, so the compiler rejects any second reference to
    /// `cell` — and therefore any second node tag — while this one lives.
    ///
    /// Use this to place cells that live on the stack or inside a caller-owned
    /// buffer; [`NumaPinnedCell`] covers the owned case.
    ///
    /// ```
    /// use themis::{NumaNodeId, NumaPinnedCellRef, sync_region_placement_scope};
    ///
    /// sync_region_placement_scope(|placement| {
    ///     let mut cell = placement.cell(7u32);
    ///     let pinned = NumaPinnedCellRef::from_unique(NumaNodeId::new(0), &mut cell);
    ///     assert_eq!(pinned.node_id(), NumaNodeId::new(0));
    /// });
    /// ```
    #[must_use]
    #[inline]
    pub const fn from_unique(node_id: NumaNodeId, cell: &'a mut MelinoeCell<'brand, T>) -> Self {
        Self { node_id, cell }
    }

    /// Returns the pinned NUMA node ID.
    #[must_use]
    #[inline]
    pub const fn node_id(&self) -> NumaNodeId {
        self.node_id
    }
}

// SAFETY: the borrowed cell reached this wrapper either through
// `from_unique`, which consumes an exclusive borrow for `'a` and so precludes a
// second wrapper over the same cell, or through
// `NumaPinnedCell::as_pinned_ref`, which copies the owner's tag rather than
// accepting one. Either way the cell answers to exactly this `node_id`.
unsafe impl<'brand, T> PinnedCell<'brand, T> for NumaPinnedCellRef<'_, 'brand, T> {
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

    /// Borrows these cells as a slice reference carrying the same pin.
    ///
    /// The tag is inherited from the owner rather than supplied by the caller.
    #[must_use]
    #[inline]
    pub const fn as_pinned_ref(&self) -> NumaPinnedSliceRef<'_, 'brand, T> {
        NumaPinnedSliceRef {
            node_id: self.node_id,
            cells: &self.cells,
        }
    }
}

// SAFETY: the cells are owned by this struct and produced by its constructors,
// so no other pinned wrapper can name them; `node_id` returns a `Copy` field
// fixed at construction.
unsafe impl<'brand, T> PinnedSlice<'brand, T> for NumaPinnedSlice<'brand, T> {
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
    /// Pins an exclusively borrowed cell slice to `node_id`.
    ///
    /// The `&mut` borrow is the placement proof: it is consumed for `'a`, so no
    /// second wrapper — and no second node tag — can cover these cells while
    /// this one lives. Placing a stack array needs no allocation:
    ///
    /// ```
    /// use melinoe::MelinoeCell;
    /// use themis::{NumaNodeId, NumaPinnedSliceRef, sync_region_placement_scope};
    ///
    /// sync_region_placement_scope(|placement| {
    ///     let mut cells = [placement.cell(1u32), placement.cell(2u32)];
    ///     let pinned = NumaPinnedSliceRef::from_unique(NumaNodeId::new(0), &mut cells);
    ///     assert_eq!(pinned.node_id(), NumaNodeId::new(0));
    /// });
    /// ```
    #[must_use]
    #[inline]
    pub const fn from_unique(node_id: NumaNodeId, cells: &'a mut [MelinoeCell<'brand, T>]) -> Self {
        Self { node_id, cells }
    }

    /// Returns the pinned NUMA node ID.
    #[must_use]
    #[inline]
    pub const fn node_id(&self) -> NumaNodeId {
        self.node_id
    }
}

// SAFETY: the borrowed cells reached this wrapper either through
// `from_unique`, which consumes an exclusive borrow for `'a` and so precludes a
// second wrapper over the same cells, or through
// `NumaPinnedSlice::as_pinned_ref`, which copies the owner's tag rather than
// accepting one. Either way every cell answers to exactly this `node_id`.
unsafe impl<'brand, T> PinnedSlice<'brand, T> for NumaPinnedSliceRef<'_, 'brand, T> {
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
