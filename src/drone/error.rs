//! Error types for drone session management.

use crate::unit::UnitId;

/// Indicates that a drone session could not be created because one already exists.
#[derive(Debug, thiserror::Error)]
#[error("drone {unit_id} already has an active session")]
pub struct SessionAlreadyActive {
    pub unit_id: UnitId,
}

/// Indicates that a drone session could not be found for removal or query.
#[derive(Debug, thiserror::Error)]
#[error("no active session for drone {unit_id}")]
pub struct SessionNotFound {
    pub unit_id: UnitId,
}
