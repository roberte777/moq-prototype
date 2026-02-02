use super::UnitId;

/// Indicates that an operation to create a unit failed because one already exists.
#[derive(Debug, thiserror::Error)]
#[error("the provided unit id ({unit_id}) is already present")]
pub struct UnitAlreadyPresent {
    pub unit_id: UnitId,
}

/// Indicates that an operation to retrieve a unit failed because it doesn't exist.
#[derive(Debug, thiserror::Error)]
#[error("the provided unit id ({unit_id}) could not be found")]
pub struct UnitNotFound {
    pub unit_id: UnitId,
}
