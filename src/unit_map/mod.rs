use std::sync::Arc;

pub use crate::unit::UnitId;
use dashmap::{DashMap, Entry};

use self::{
    error::{UnitAlreadyPresent, UnitNotFound},
    unit_ref::UnitRef,
};

pub mod error;
pub mod unit_ref;

/// A map of units identified by a [`UnitId`] and their associated context `T`.
///
/// When a unit is added to the map it is turned into a shared resource for which only references
/// can be obtained through the [`UnitRef`] interface.
///
/// Direct access to the strong reference is not allowed in order to prevent long lived upgrades
/// undermining lifecycle control from the [`UnitMap`].
#[derive(Debug)]
pub struct UnitMap<T> {
    entity_map: DashMap<UnitId, Arc<T>, ahash::RandomState>,
}

impl<T> UnitMap<T> {
    /// Construct a new empty [`UnitMap`].
    pub fn new() -> UnitMap<T> {
        Self::default()
    }

    /// Create a unit entity entry tracked by the `unit_id` and associated with the `unit_context`.
    pub fn insert_unit(&self, unit_id: UnitId, unit_context: T) -> Result<(), UnitAlreadyPresent> {
        match self.entity_map.entry(unit_id) {
            Entry::Occupied(entry) => Err(UnitAlreadyPresent {
                unit_id: entry.key().clone(),
            }),

            Entry::Vacant(slot) => {
                slot.insert(Arc::new(unit_context));
                Ok(())
            }
        }
    }

    /// Remove the unit entity for the provided `unit_id`.
    pub fn remove_unit(&self, unit_id: &UnitId) -> Result<(), UnitNotFound> {
        self.entity_map
            .remove(unit_id)
            .ok_or_else(|| UnitNotFound {
                unit_id: unit_id.clone(),
            })?;

        Ok(())
    }

    /// Lend the unit context for the provided `unit_id`.
    ///
    /// If the unit is present returns a [`UnitRef`] containing the unit context `T`.
    pub fn get_unit(&self, unit_id: &UnitId) -> Result<UnitRef<T>, UnitNotFound> {
        self.entity_map
            .view(unit_id, |_, entity| {
                UnitRef::new(unit_id.clone(), Arc::downgrade(entity))
            })
            .ok_or_else(|| UnitNotFound {
                unit_id: unit_id.clone(),
            })
    }
}

impl<T> Default for UnitMap<T> {
    fn default() -> Self {
        Self {
            entity_map: DashMap::default(),
        }
    }
}
