use super::StateMachine;

pub struct TelemetryMachine {
    latest_position: Option<Position>,
    pending: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub drone_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f64,
    pub heading_deg: f64,
    pub speed_mps: f64,
    pub timestamp: u64,
}

impl TelemetryMachine {
    pub fn new() -> Self {
        Self {
            latest_position: None,
            pending: false,
        }
    }

    pub fn current_position(&self) -> Option<&Position> {
        self.latest_position.as_ref()
    }

    pub fn has_pending(&self) -> bool {
        self.pending
    }

    fn update_position(&mut self, pos: Position) {
        self.latest_position = Some(pos);
        self.pending = true;
    }

    fn poll_position(&mut self) -> Option<Position> {
        if self.pending {
            self.pending = false;
            self.latest_position.clone()
        } else {
            None
        }
    }
}

impl Default for TelemetryMachine {
    fn default() -> Self {
        Self::new()
    }
}

pub enum TelemetryInput {
    Position(Position),
}

pub enum TelemetryOutput {
    PositionUpdate(Position),
}

impl StateMachine for TelemetryMachine {
    type Input = TelemetryInput;
    type Output = TelemetryOutput;

    fn process_input(&mut self, input: Self::Input) {
        match input {
            TelemetryInput::Position(pos) => self.update_position(pos),
        }
    }

    fn poll_output(&mut self) -> Option<Self::Output> {
        self.poll_position().map(TelemetryOutput::PositionUpdate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_position(lat: f64) -> Position {
        Position {
            drone_id: "test-drone".to_string(),
            latitude: lat,
            longitude: -122.4194,
            altitude_m: 100.0,
            heading_deg: 0.0,
            speed_mps: 0.0,
            timestamp: 1234567890,
        }
    }

    #[test]
    fn test_initial_state() {
        let machine = TelemetryMachine::new();
        assert!(machine.current_position().is_none());
        assert!(!machine.has_pending());
    }

    #[test]
    fn test_update_and_poll() {
        let mut machine = TelemetryMachine::new();

        let pos = sample_position(37.7749);
        machine.process_input(TelemetryInput::Position(pos.clone()));

        assert!(machine.has_pending());
        assert_eq!(machine.current_position(), Some(&pos));

        // Poll consumes the pending flag
        let output = machine.poll_output();
        assert!(matches!(output, Some(TelemetryOutput::PositionUpdate(ref p)) if p == &pos));

        // No longer pending, but position is still available
        assert!(!machine.has_pending());
        assert_eq!(machine.current_position(), Some(&pos));

        // Second poll returns None
        assert!(machine.poll_output().is_none());
    }

    #[test]
    fn test_latest_position_overwrites() {
        let mut machine = TelemetryMachine::new();

        machine.process_input(TelemetryInput::Position(sample_position(37.0)));
        machine.process_input(TelemetryInput::Position(sample_position(38.0)));

        // Only the latest position is available
        let output = machine.poll_output();
        assert!(matches!(output, Some(TelemetryOutput::PositionUpdate(p)) if p.latitude == 38.0));
    }
}
