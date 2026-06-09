//! Melinoe-backed branded placement scopes.

use melinoe::sync::{sync_region_scope, thread_local_scope, SyncRegionToken, ThreadLocalToken};
use melinoe::{MelinoeCell, MelinoeMut, MelinoeRef};

/// Branded storage for placement state.
pub type PlacementCell<'brand, T> = MelinoeCell<'brand, T>;

/// Thread-confined placement capability.
///
/// The inner Melinoe token is `!Send + !Sync`, so placement state accessed
/// through this scope cannot cross thread boundaries.
pub struct ThreadLocalPlacement<'brand> {
    token: ThreadLocalToken<'brand>,
}

impl<'brand> ThreadLocalPlacement<'brand> {
    /// Creates a placement cell in this brand.
    #[must_use]
    #[inline]
    pub const fn cell<T>(&self, value: T) -> PlacementCell<'brand, T> {
        PlacementCell::new(value)
    }

    /// Reads placement state through the thread-confined permit.
    #[inline]
    pub fn read<'a, T>(&'a self, cell: &'a PlacementCell<'brand, T>) -> MelinoeRef<'a, 'brand, T> {
        cell.borrow(&self.token)
    }

    /// Writes placement state through the thread-confined permit.
    #[inline]
    pub fn write<'a, T>(
        &'a mut self,
        cell: &'a PlacementCell<'brand, T>,
    ) -> MelinoeMut<'a, 'brand, T> {
        cell.borrow_mut(&mut self.token)
    }
}

/// Thread-portable placement-region capability.
///
/// The inner Melinoe token is `Send + Sync`, so a complete placement region may
/// be moved between execution domains while preserving single-writer discipline.
pub struct SyncRegionPlacement<'brand> {
    token: SyncRegionToken<'brand>,
}

impl<'brand> SyncRegionPlacement<'brand> {
    /// Creates a placement cell in this brand.
    #[must_use]
    #[inline]
    pub const fn cell<T>(&self, value: T) -> PlacementCell<'brand, T> {
        PlacementCell::new(value)
    }

    /// Reads placement state through the sync-region permit.
    #[inline]
    pub fn read<'a, T>(&'a self, cell: &'a PlacementCell<'brand, T>) -> MelinoeRef<'a, 'brand, T> {
        cell.borrow(&self.token)
    }

    /// Writes placement state through the sync-region permit.
    #[inline]
    pub fn write<'a, T>(
        &'a mut self,
        cell: &'a PlacementCell<'brand, T>,
    ) -> MelinoeMut<'a, 'brand, T> {
        cell.borrow_mut(&mut self.token)
    }
}

/// Opens a thread-confined placement scope.
#[inline]
pub fn thread_local_placement_scope<R>(
    f: impl for<'brand> FnOnce(ThreadLocalPlacement<'brand>) -> R,
) -> R {
    thread_local_scope(|token| f(ThreadLocalPlacement { token }))
}

/// Opens a thread-portable placement-region scope.
#[inline]
pub fn sync_region_placement_scope<R>(
    f: impl for<'brand> FnOnce(SyncRegionPlacement<'brand>) -> R,
) -> R {
    sync_region_scope(|token| f(SyncRegionPlacement { token }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryTier, NumaNodeId, PlacementHint};

    #[test]
    fn thread_local_scope_controls_placement_state() {
        let observed = thread_local_placement_scope(|mut placement| {
            let hint = placement.cell(PlacementHint::Current);
            *placement.write(&hint) = PlacementHint::Numa(NumaNodeId::new(3));
            *placement.read(&hint)
        });

        assert_eq!(observed, PlacementHint::Numa(NumaNodeId::new(3)));
    }

    #[test]
    fn sync_region_scope_controls_portable_placement_state() {
        let observed = sync_region_placement_scope(|mut placement| {
            let tier = placement.cell(MemoryTier::Dram);
            *placement.write(&tier) = MemoryTier::Hbm;
            *placement.read(&tier)
        });

        assert_eq!(observed, MemoryTier::Hbm);
    }
}
