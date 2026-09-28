//! Shared storage for the branded placement wrappers.
//!
//! The dynamic (`NumaPinned*`) and static (`ConstNumaPinned*`) families differ
//! only in how the NUMA node tag is carried: the dynamic family stores a
//! [`NumaNodeId`] value, the static family encodes it as a const generic on a
//! zero-sized [`ConstNodeTag`]. Both families are therefore one storage type,
//! [`PinnedStorage`], parameterised by that tag and by the container it holds —
//! an owned or borrowed cell, or an owned or borrowed slice of cells.

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "std")]
use std::{boxed::Box, vec::Vec};

use melinoe::{collections::BrandedVec, MelinoeCell};

use super::cell::{ConstPinnedCell, ConstPinnedSlice, PinnedCell, PinnedSlice};
use crate::NumaNodeId;

/// A zero-sized NUMA node tag that encodes its node id in the type.
///
/// This is the tag parameter of [`PinnedStorage`] for the statically pinned
/// `ConstNumaPinned*` family; the dynamic `NumaPinned*` family uses a plain
/// [`NumaNodeId`] as its tag instead. Being a distinct type per `NODE_ID`, it
/// keeps a statically pinned cell from answering to any other tag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConstNodeTag<const NODE_ID: u32>;

/// A container that exposes exactly one branded cell by shared reference.
///
/// Implemented by an owned [`MelinoeCell`] and by an exclusive `&MelinoeCell`
/// borrow, so the owned and borrowed pinned-cell wrappers share one storage
/// type and one set of constructors.
pub trait CellContainer<'brand, T> {
    /// Returns the contained cell.
    fn as_cell(&self) -> &MelinoeCell<'brand, T>;
}

impl<'brand, T> CellContainer<'brand, T> for MelinoeCell<'brand, T> {
    #[inline]
    fn as_cell(&self) -> &MelinoeCell<'brand, T> {
        self
    }
}

impl<'brand, T> CellContainer<'brand, T> for &MelinoeCell<'brand, T> {
    #[inline]
    fn as_cell(&self) -> &MelinoeCell<'brand, T> {
        self
    }
}

/// A container that exposes a contiguous slice of branded cells.
///
/// Implemented by an owned boxed slice of cells and by an exclusive
/// `&[MelinoeCell]` borrow, so the owned and borrowed pinned-slice wrappers
/// share one storage type and one set of constructors.
pub trait CellSlice<'brand, T> {
    /// Returns the contained cells.
    fn as_cells(&self) -> &[MelinoeCell<'brand, T>];
}

impl<'brand, T> CellSlice<'brand, T> for Box<[MelinoeCell<'brand, T>]> {
    #[inline]
    fn as_cells(&self) -> &[MelinoeCell<'brand, T>] {
        self
    }
}

impl<'brand, T> CellSlice<'brand, T> for &[MelinoeCell<'brand, T>] {
    #[inline]
    fn as_cells(&self) -> &[MelinoeCell<'brand, T>] {
        self
    }
}

/// Storage shared by every pinned placement wrapper.
///
/// `Tag` is the node tag ([`NumaNodeId`] for the dynamic family,
/// [`ConstNodeTag`] for the static family) and `Storage` is the container: a
/// [`MelinoeCell`], a `&MelinoeCell` borrow, a boxed slice of cells, or a
/// borrowed slice of cells.
pub struct PinnedStorage<Tag, Storage> {
    tag: Tag,
    storage: Storage,
}

impl<Storage> PinnedStorage<NumaNodeId, Storage> {
    /// Returns the pinned NUMA node ID.
    #[must_use]
    #[inline]
    pub const fn node_id(&self) -> NumaNodeId {
        self.tag
    }
}

impl<T> PinnedStorage<NumaNodeId, MelinoeCell<'_, T>> {
    /// Creates a new cell pinned to the specified NUMA node.
    #[must_use]
    pub const fn new(node_id: NumaNodeId, value: T) -> Self {
        Self {
            tag: node_id,
            storage: MelinoeCell::new(value),
        }
    }
}

impl<const NODE_ID: u32, T> PinnedStorage<ConstNodeTag<NODE_ID>, MelinoeCell<'_, T>> {
    /// Creates a new cell pinned statically to the node ID.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self {
            tag: ConstNodeTag,
            storage: MelinoeCell::new(value),
        }
    }
}

impl<'a, 'brand, T> PinnedStorage<NumaNodeId, &'a MelinoeCell<'brand, T>> {
    /// Pins an exclusively borrowed cell to `node_id`.
    ///
    /// The `&mut` borrow is the placement proof and the reason this is safe:
    /// it is consumed for `'a`, so the compiler rejects any second reference to
    /// `cell` — and therefore any second node tag — while this one lives.
    ///
    /// Use this to place cells that live on the stack or inside a caller-owned
    /// buffer; the owned constructor covers the owned case.
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
    pub fn from_unique(node_id: NumaNodeId, cell: &'a mut MelinoeCell<'brand, T>) -> Self {
        Self {
            tag: node_id,
            storage: cell,
        }
    }
}

impl<'a, 'brand, const NODE_ID: u32, T>
    PinnedStorage<ConstNodeTag<NODE_ID>, &'a MelinoeCell<'brand, T>>
{
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
    pub fn from_unique(cell: &'a mut MelinoeCell<'brand, T>) -> Self {
        Self {
            tag: ConstNodeTag,
            storage: cell,
        }
    }
}

impl<'brand, T> PinnedStorage<NumaNodeId, Box<[MelinoeCell<'brand, T>]>> {
    /// Creates a new pinned slice from a vector of values.
    #[must_use]
    pub fn new(node_id: NumaNodeId, values: Vec<T>) -> Self {
        Self {
            tag: node_id,
            storage: BrandedVec::from_iter(values).into_boxed_cells(),
        }
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
        Self {
            tag: node_id,
            storage: BrandedVec::from_fn(len, generate).into_boxed_cells(),
        }
    }

    /// Creates a new pinned slice directly from a boxed slice of cells.
    #[must_use]
    pub const fn from_cells(node_id: NumaNodeId, cells: Box<[MelinoeCell<'brand, T>]>) -> Self {
        Self {
            tag: node_id,
            storage: cells,
        }
    }
}

impl<'brand, const NODE_ID: u32, T>
    PinnedStorage<ConstNodeTag<NODE_ID>, Box<[MelinoeCell<'brand, T>]>>
{
    /// Creates a new pinned slice from a vector of values.
    #[must_use]
    pub fn new(values: Vec<T>) -> Self {
        Self {
            tag: ConstNodeTag,
            storage: BrandedVec::from_iter(values).into_boxed_cells(),
        }
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
        Self {
            tag: ConstNodeTag,
            storage: BrandedVec::from_fn(len, generate).into_boxed_cells(),
        }
    }

    /// Creates a new pinned slice directly from a boxed slice of cells.
    #[must_use]
    pub const fn from_cells(cells: Box<[MelinoeCell<'brand, T>]>) -> Self {
        Self {
            tag: ConstNodeTag,
            storage: cells,
        }
    }
}

impl<'a, 'brand, T> PinnedStorage<NumaNodeId, &'a [MelinoeCell<'brand, T>]> {
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
    pub fn from_unique(node_id: NumaNodeId, cells: &'a mut [MelinoeCell<'brand, T>]) -> Self {
        Self {
            tag: node_id,
            storage: cells,
        }
    }
}

impl<'a, 'brand, const NODE_ID: u32, T>
    PinnedStorage<ConstNodeTag<NODE_ID>, &'a [MelinoeCell<'brand, T>]>
{
    /// Pins an exclusively borrowed cell slice to `NODE_ID`.
    ///
    /// The `&mut` borrow is the placement proof: it is consumed for `'a`, so no
    /// second wrapper — and no second `NODE_ID` — can cover these cells while
    /// this one lives. Placing a stack array needs no allocation.
    #[must_use]
    #[inline]
    pub fn from_unique(cells: &'a mut [MelinoeCell<'brand, T>]) -> Self {
        Self {
            tag: ConstNodeTag,
            storage: cells,
        }
    }
}

impl<'brand, Tag: Copy, T> PinnedStorage<Tag, MelinoeCell<'brand, T>> {
    /// Borrows this cell as a reference carrying the same pin.
    ///
    /// The tag is inherited from the owner rather than supplied by the caller,
    /// so the reference cannot relabel the cell onto another NUMA node.
    #[must_use]
    #[inline]
    pub const fn as_pinned_ref(&self) -> PinnedStorage<Tag, &MelinoeCell<'brand, T>> {
        PinnedStorage {
            tag: self.tag,
            storage: &self.storage,
        }
    }
}

impl<'brand, Tag, T> PinnedStorage<Tag, Box<[MelinoeCell<'brand, T>]>> {
    /// Borrow the underlying cells immutably.
    #[must_use]
    #[inline]
    pub fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        &self.storage
    }
}

impl<'brand, Tag: Copy, T> PinnedStorage<Tag, Box<[MelinoeCell<'brand, T>]>> {
    /// Borrows these cells as a slice reference carrying the same pin.
    ///
    /// The tag is inherited from the owner rather than supplied by the caller.
    #[must_use]
    #[inline]
    pub const fn as_pinned_ref(&self) -> PinnedStorage<Tag, &[MelinoeCell<'brand, T>]> {
        PinnedStorage {
            tag: self.tag,
            storage: &self.storage,
        }
    }
}

#[cfg(feature = "std")]
impl<'brand, Tag, T> PinnedStorage<Tag, Box<[MelinoeCell<'brand, T>]>> {
    /// Access the uniquely owned branded cells for Themis's placement-gated
    /// Melinoe partition driver.
    pub(crate) fn cells_mut(&mut self) -> &mut [MelinoeCell<'brand, T>] {
        &mut self.storage
    }
}

// SAFETY — the dynamic (`NumaNodeId`) cell family, one impl covering both the
// owned and borrowed containers. An owned `MelinoeCell` is created by
// `PinnedStorage::new` and named by no other pinned wrapper, so its tag is
// exclusively this one. A `&MelinoeCell` container is only ever built by
// `from_unique`, which consumes an exclusive borrow for the wrapper's whole
// life, or by `as_pinned_ref`, which copies the owner's tag rather than
// accepting one. Either way the cell answers to exactly this `node_id`, which
// is a `Copy` value fixed at construction.
unsafe impl<'brand, T, Storage: CellContainer<'brand, T>> PinnedCell<'brand, T>
    for PinnedStorage<NumaNodeId, Storage>
{
    #[inline]
    fn node_id(&self) -> NumaNodeId {
        self.tag
    }

    #[inline]
    fn cell(&self) -> &MelinoeCell<'brand, T> {
        self.storage.as_cell()
    }
}

// SAFETY — as the `PinnedCell` impl, with `NODE_ID` as the tag: an owned cell
// is named by no other wrapper, a borrowed one reaches here only through
// `from_unique` (an exclusive borrow) or `as_pinned_ref` (an inherited tag),
// and `NODE_ID` is part of the type, so the tag cannot vary between calls.
unsafe impl<'brand, const NODE_ID: u32, T, Storage: CellContainer<'brand, T>>
    ConstPinnedCell<'brand, NODE_ID, T> for PinnedStorage<ConstNodeTag<NODE_ID>, Storage>
{
    #[inline]
    fn cell(&self) -> &MelinoeCell<'brand, T> {
        self.storage.as_cell()
    }
}

// SAFETY — the dynamic slice family, applied elementwise as `PinnedCell`: the
// cells are owned by this wrapper (and produced by its constructors, so no
// other wrapper can name them) or borrowed exclusively for the wrapper's whole
// life, and `node_id` is a `Copy` value fixed at construction.
unsafe impl<'brand, T, Storage: CellSlice<'brand, T>> PinnedSlice<'brand, T>
    for PinnedStorage<NumaNodeId, Storage>
{
    #[inline]
    fn node_id(&self) -> NumaNodeId {
        self.tag
    }

    #[inline]
    fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        self.storage.as_cells()
    }
}

// SAFETY — as the `PinnedSlice` impl, with `NODE_ID` as the tag: ownership or an
// exclusive borrow keeps every cell reachable through exactly this wrapper, and
// `NODE_ID` being part of the type keeps the tag constant.
unsafe impl<'brand, const NODE_ID: u32, T, Storage: CellSlice<'brand, T>>
    ConstPinnedSlice<'brand, NODE_ID, T> for PinnedStorage<ConstNodeTag<NODE_ID>, Storage>
{
    #[inline]
    fn cells(&self) -> &[MelinoeCell<'brand, T>] {
        self.storage.as_cells()
    }
}
