pub mod error;

use crate::unit::UnitId;
use dashmap::{DashMap, Entry};
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

use self::error::{SessionAlreadyActive, SessionNotFound};

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct DroneSessionId(Arc<Uuid>);

impl DroneSessionId {
    pub fn generate() -> Self {
        Self(Arc::new(Uuid::new_v4()))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Debug for DroneSessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DroneSessionId({})", self.0)
    }
}

impl fmt::Display for DroneSessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct DroneSession {
    pub session_id: DroneSessionId,
    pub unit_id: UnitId,
}

#[derive(Debug)]
pub struct DroneSessionMap {
    sessions: DashMap<UnitId, DroneSession, ahash::RandomState>,
}

impl DroneSessionMap {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::default(),
        }
    }

    pub fn create_session(&self, unit_id: &UnitId) -> Result<DroneSessionId, SessionAlreadyActive> {
        match self.sessions.entry(unit_id.clone()) {
            Entry::Occupied(_) => Err(SessionAlreadyActive {
                unit_id: unit_id.clone(),
            }),
            Entry::Vacant(slot) => {
                let session_id = DroneSessionId::generate();
                slot.insert(DroneSession {
                    session_id: session_id.clone(),
                    unit_id: unit_id.clone(),
                });
                Ok(session_id)
            }
        }
    }

    pub fn remove_session(&self, unit_id: &UnitId) -> Result<DroneSession, SessionNotFound> {
        self.sessions
            .remove(unit_id)
            .map(|(_, session)| session)
            .ok_or_else(|| SessionNotFound {
                unit_id: unit_id.clone(),
            })
    }

    pub fn has_active_session(&self, unit_id: &UnitId) -> bool {
        self.sessions.contains_key(unit_id)
    }

    pub fn get_session_id(&self, unit_id: &UnitId) -> Option<DroneSessionId> {
        self.sessions
            .get(unit_id)
            .map(|entry| entry.session_id.clone())
    }

    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for DroneSessionMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let map = DroneSessionMap::new();
        let unit_id = UnitId::from("drone-1");

        let result = map.create_session(&unit_id);
        assert!(result.is_ok());
        assert!(map.has_active_session(&unit_id));
        assert_eq!(map.active_session_count(), 1);
    }

    #[test]
    fn test_duplicate_session_error() {
        let map = DroneSessionMap::new();
        let unit_id = UnitId::from("drone-1");

        let _ = map.create_session(&unit_id).unwrap();

        // Second attempt should fail
        let result = map.create_session(&unit_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionAlreadyActive { .. }));
    }

    #[test]
    fn test_remove_session() {
        let map = DroneSessionMap::new();
        let unit_id = UnitId::from("drone-1");

        let session_id = map.create_session(&unit_id).unwrap();

        let removed = map.remove_session(&unit_id).unwrap();
        assert_eq!(removed.session_id, session_id);
        assert!(!map.has_active_session(&unit_id));
    }

    #[test]
    fn test_remove_nonexistent_session() {
        let map = DroneSessionMap::new();
        let unit_id = UnitId::from("drone-1");

        let result = map.remove_session(&unit_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionNotFound { .. }));
    }

    #[test]
    fn test_reconnect_after_disconnect() {
        let map = DroneSessionMap::new();
        let unit_id = UnitId::from("drone-1");

        // First connection
        let _ = map.create_session(&unit_id).unwrap();
        let _ = map.remove_session(&unit_id).unwrap();

        // Reconnect should succeed
        let result = map.create_session(&unit_id);
        assert!(result.is_ok());
    }
}
