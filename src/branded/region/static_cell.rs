//! Statically tagged branded placement cells.
//!
//! The concrete wrapper names are aliases over
//! [`PinnedStorage`](super::storage::PinnedStorage) with a zero-sized
//! [`ConstNodeTag`] tag, which encodes the NUMA node id in the type. The
//! dynamically tagged family lives in the sibling [`cell`](super::cell)
//! module; both share one storage type and one set of implementations.

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[cfg(feature = "std")]
use std::boxed::Box;

use melinoe::MelinoeCell;

use super::storage::{ConstNodeTag, PinnedStorage};

/// A placement cell pinned to a specific NUMA node, verified at compile time.
pub type ConstNumaPinnedCell<'brand, const NODE_ID: u32, T> =
    PinnedStorage<ConstNodeTag<NODE_ID>, MelinoeCell<'brand, T>>;

/// A borrowed reference to a cell pinned statically to a specific NUMA node.
pub type ConstNumaPinnedCellRef<'a, 'brand, const NODE_ID: u32, T> =
    PinnedStorage<ConstNodeTag<NODE_ID>, &'a MelinoeCell<'brand, T>>;

/// A contiguous slice of cells pinned to a specific NUMA node, verified at compile time.
pub type ConstNumaPinnedSlice<'brand, const NODE_ID: u32, T> =
    PinnedStorage<ConstNodeTag<NODE_ID>, Box<[MelinoeCell<'brand, T>]>>;

/// A borrowed reference to a contiguous slice of cells pinned statically to a specific NUMA node.
pub type ConstNumaPinnedSliceRef<'a, 'brand, const NODE_ID: u32, T> =
    PinnedStorage<ConstNodeTag<NODE_ID>, &'a [MelinoeCell<'brand, T>]>;
