//! Targets without a thread-binding backend, and interpreters that cannot
//! issue the calls. No value of [`Mask`] exists, so every query on one is
//! unreachable by construction and every constructor reports
//! [`BindError::Unsupported`].

use core::convert::Infallible;
use core::iter::Empty;

use super::BindError;

#[derive(Clone, Eq, Hash, PartialEq)]
pub(super) struct Mask(Infallible);

impl Mask {
    pub(super) fn from_processors(
        _processors: impl Iterator<Item = u32>,
    ) -> Result<Self, BindError> {
        Err(BindError::Unsupported)
    }

    pub(super) fn contains(&self, _processor: u32) -> bool {
        match self.0 {}
    }

    pub(super) fn len(&self) -> u32 {
        match self.0 {}
    }

    pub(super) fn processors(&self) -> Empty<u32> {
        match self.0 {}
    }
}

pub(super) fn current() -> Result<Mask, BindError> {
    Err(BindError::Unsupported)
}

pub(super) fn apply(mask: &Mask) -> Result<(), BindError> {
    match mask.0 {}
}
