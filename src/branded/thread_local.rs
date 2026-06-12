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
}

/// Opens a thread-confined placement scope.
#[inline]
pub fn thread_local_placement_scope<R>(
    f: impl for<'brand> FnOnce(ThreadLocalPlacement<'brand>) -> R,
) -> R {
    thread_local_scope(|token| f(ThreadLocalPlacement { token }))
}
