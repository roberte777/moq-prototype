use std::sync::Mutex;

use crate::state_machine::StateMachine;
use crate::state_machine::command::{CommandInput, CommandOutput, CommandQueueMachine};
use crate::state_machine::telemetry::{
    Position, TelemetryInput, TelemetryMachine, TelemetryOutput,
};

pub struct UnitContext {
    command_machine: Mutex<CommandQueueMachine>,
    telemetry_machine: Mutex<TelemetryMachine>,
}

impl std::fmt::Debug for UnitContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnitContext")
            .field("command_machine", &"<CommandQueueMachine>")
            .field("telemetry_machine", &"<TelemetryMachine>")
            .finish()
    }
}

impl UnitContext {
    pub fn new() -> Self {
        Self {
            command_machine: Mutex::new(CommandQueueMachine::new()),
            telemetry_machine: Mutex::new(TelemetryMachine::new()),
        }
    }

    pub fn enqueue_command(&self, cmd: Vec<u8>) {
        let mut machine = self
            .command_machine
            .lock()
            .expect("command machine lock poisoned");
        machine.process_input(CommandInput::Enqueue(cmd));
    }

    pub fn poll_command(&self) -> Option<Vec<u8>> {
        let mut machine = self
            .command_machine
            .lock()
            .expect("command machine lock poisoned");
        machine.poll_output().map(|out| match out {
            CommandOutput::Command(bytes) => bytes,
        })
    }

    pub fn update_telemetry(&self, pos: Position) {
        let mut machine = self
            .telemetry_machine
            .lock()
            .expect("telemetry machine lock poisoned");
        machine.process_input(TelemetryInput::Position(pos));
    }

    pub fn poll_telemetry(&self) -> Option<Position> {
        let mut machine = self
            .telemetry_machine
            .lock()
            .expect("telemetry machine lock poisoned");
        machine.poll_output().map(|out| match out {
            TelemetryOutput::PositionUpdate(pos) => pos,
        })
    }
}

impl Default for UnitContext {
    fn default() -> Self {
        Self::new()
    }
}
