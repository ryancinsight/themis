//! Dynamically tagged branded placement cells.
//!
//! The concrete wrapper names are aliases over [`PinnedStorage`] with a runtime
//! [`NumaNodeId`] tag. The statically tagged family, whose tag is a zero-sized
//! const-generic type, lives in the sibling [`static_cell`](super::static_cell)
//! module; both share one storage type and one set of implementations.

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[cfg(feature = "std")]
use std::boxed::Box;

use melinoe::MelinoeCell;

use super::storage::PinnedStorage;
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
//
// These are aliases over `PinnedStorage` tagged with a runtime `NumaNodeId`;
// `PinnedStorage` is where every constructor, accessor, and unsafe-impl body
// lives.

/// A placement cell pinned to a specific NUMA node.
pub type NumaPinnedCell<'brand, T> = PinnedStorage<NumaNodeId, MelinoeCell<'brand, T>>;

/// A borrowed reference to a cell pinned to a specific NUMA node.
pub type NumaPinnedCellRef<'a, 'brand, T> = PinnedStorage<NumaNodeId, &'a MelinoeCell<'brand, T>>;

/// A contiguous slice of cells pinned to a specific NUMA node.
pub type NumaPinnedSlice<'brand, T> = PinnedStorage<NumaNodeId, Box<[MelinoeCell<'brand, T>]>>;

/// A borrowed reference to a contiguous slice of cells pinned to a specific NUMA node.
pub type NumaPinnedSliceRef<'a, 'brand, T> =
    PinnedStorage<NumaNodeId, &'a [MelinoeCell<'brand, T>]>;
