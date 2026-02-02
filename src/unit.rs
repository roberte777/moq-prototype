use std::fmt::Display;
use std::sync::Arc;

/// An ID for a "unit" which is a compound virtual object that is a semantic combination of
/// a drone, dock, and potentially other hardware.
///
/// Currently this can be any generic string and is notably also externally editable.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnitId(Arc<str>);

impl UnitId {
    /// Create a new [`UnitId`] from any type that can be converted into an `Arc<str>`.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    /// Returns the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for UnitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for UnitId {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl From<&str> for UnitId {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}
