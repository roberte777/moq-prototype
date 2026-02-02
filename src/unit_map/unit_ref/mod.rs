use std::{convert::Infallible, fmt, sync::Weak};

use self::error::UnitViewInvalid;
use super::UnitId;

pub mod error;

/// A weak reference to a shared unit context that provides a scoped [`view`](Self::view).
pub struct UnitRef<T> {
    unit_id: UnitId,
    weak_unit_context: Weak<T>,
}

impl<T> UnitRef<T> {
    /// Construct a new [`UnitRef`] from an identifier and weak reference to a unit context.
    pub(super) fn new(unit_id: UnitId, weak_unit_context: Weak<T>) -> UnitRef<T> {
        Self {
            unit_id,
            weak_unit_context,
        }
    }

    /// Scoped access via a `view_fn` to the `unit_context` for the unit reference.
    ///
    /// If the unit context exists returns the value `R` computed from the `view_fn`, else
    /// returns a [`UnitViewInvalid`] error indicating unit context is no longer valid.
    pub fn view<F: FnOnce(&T) -> R, R>(&self, view_fn: F) -> Result<R, UnitViewInvalid> {
        Weak::upgrade(&self.weak_unit_context)
            .map(|unit_context| view_fn(&unit_context))
            .ok_or(UnitViewInvalid {
                unit_id: self.unit_id.clone(),
            })
    }
}

#[expect(clippy::missing_fields_in_debug, reason = "custom weak handling")]
impl<T: fmt::Debug> fmt::Debug for UnitRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.view(|context| {
            f.debug_struct("UnitRef")
                .field("unit_id", &self.unit_id)
                .field("unit_context", &Ok::<_, Infallible>(context))
                .finish()
        })
        .unwrap_or_else(|err| {
            f.debug_struct("SessionRef")
                .field("session_id", &self.unit_id)
                .field("unit_context", &Err::<Infallible, _>(err))
                .finish()
        })
    }
}

impl<T> Clone for UnitRef<T> {
    fn clone(&self) -> Self {
        Self {
            unit_id: self.unit_id.clone(),
            weak_unit_context: self.weak_unit_context.clone(),
        }
    }
}

/// A [`UnitRef`] is defined only by the underlying [`UnitId`] matching.
impl<T> PartialEq<UnitId> for UnitRef<T> {
    fn eq(&self, other: &UnitId) -> bool {
        self.unit_id == *other
    }
}

/// A [`UnitRef`] is defined only by the underlying [`UnitId`] matching.
impl<T> PartialEq<UnitRef<T>> for UnitId {
    fn eq(&self, other: &UnitRef<T>) -> bool {
        *self == other.unit_id
    }
}
