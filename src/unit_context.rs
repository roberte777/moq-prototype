use std::sync::Mutex;

use crate::state_machine::{
    StateMachine,
    echo::{EchoInput, EchoMachine, EchoOutput, Position},
};

#[derive(Debug)]
pub struct UnitContext {
    echo: Mutex<EchoMachine>,
}

impl UnitContext {
    pub fn new() -> Self {
        Self {
            echo: Mutex::new(EchoMachine::new()),
        }
    }

    // TODO: Make a view type instead of passing through to the state machine here
    pub fn update_position(&self, pos: Position) {
        let mut machine = self.echo.lock().expect("telemetry machine lock poisoned");
        machine.process_input(EchoInput::Position(pos));
    }

    pub fn poll_position(&self) -> Option<Position> {
        let mut machine = self.echo.lock().expect("telemetry machine lock poisoned");
        machine.poll_output().map(|out| match out {
            EchoOutput::Position(pos) => pos,
        })
    }
}

impl Default for UnitContext {
    fn default() -> Self {
        Self::new()
    }
}
