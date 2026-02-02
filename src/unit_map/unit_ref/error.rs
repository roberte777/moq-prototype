use super::UnitId;

/// Indicates that an operation to view a weakly held unit context failed because the owner of the
/// context has destroyed it.
#[derive(Debug, thiserror::Error)]
#[error("the unit context ({unit_id}) is no longer valid")]
pub struct UnitViewInvalid {
    pub unit_id: UnitId,
}
