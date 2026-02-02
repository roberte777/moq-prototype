use moq_lite::{BroadcastProducer, TrackProducer};
use std::sync::Mutex;

pub struct UnitContext {
    broadcast_producer: BroadcastProducer,
    command_track: Mutex<TrackProducer>,
}

impl std::fmt::Debug for UnitContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnitContext")
            .field("broadcast_producer", &"<BroadcastProducer>")
            .field("command_track", &"<TrackProducer>")
            .finish()
    }
}

impl UnitContext {
    /// Create a new [`UnitContext`] with the given broadcast and command track producers.
    pub fn new(broadcast_producer: BroadcastProducer, command_track: TrackProducer) -> Self {
        Self {
            broadcast_producer,
            command_track: Mutex::new(command_track),
        }
    }

    /// Returns a reference to the broadcast producer.
    pub fn broadcast_producer(&self) -> &BroadcastProducer {
        &self.broadcast_producer
    }

    /// Write a frame to the command track.
    pub fn write_command_frame<B: Into<bytes::Bytes>>(&self, frame: B) {
        let mut track = self
            .command_track
            .lock()
            .expect("command track lock poisoned");
        track.write_frame(frame);
    }
}
