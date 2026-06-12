//! Thread-portable branded placement scopes.

use melinoe::sync::{sync_region_scope, SyncRegionToken};
use melinoe::{MelinoeCell, MelinoeMut, MelinoeRef};

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
}

/// Opens a thread-portable placement-region scope.
#[inline]
pub fn sync_region_placement_scope<R>(
    f: impl for<'brand> FnOnce(SyncRegionPlacement<'brand>) -> R,
) -> R {
    sync_region_scope(|token| f(SyncRegionPlacement { token }))
}
