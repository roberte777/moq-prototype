use std::collections::VecDeque;

use super::StateMachine;

pub struct CommandQueueMachine {
    pending_commands: VecDeque<Vec<u8>>,
}

impl CommandQueueMachine {
    pub fn new() -> Self {
        Self {
            pending_commands: VecDeque::new(),
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending_commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending_commands.is_empty()
    }

    fn enqueue(&mut self, cmd: Vec<u8>) {
        self.pending_commands.push_back(cmd);
    }

    fn dequeue(&mut self) -> Option<Vec<u8>> {
        self.pending_commands.pop_front()
    }
}

impl Default for CommandQueueMachine {
    fn default() -> Self {
        Self::new()
    }
}

pub enum CommandInput {
    Enqueue(Vec<u8>),
}

pub enum CommandOutput {
    Command(Vec<u8>),
}

impl StateMachine for CommandQueueMachine {
    type Input = CommandInput;
    type Output = CommandOutput;

    fn process_input(&mut self, input: Self::Input) {
        match input {
            CommandInput::Enqueue(cmd) => self.enqueue(cmd),
        }
    }

    fn poll_output(&mut self) -> Option<Self::Output> {
        self.dequeue().map(CommandOutput::Command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_and_poll() {
        let mut machine = CommandQueueMachine::new();

        assert!(machine.is_empty());
        assert_eq!(machine.pending_count(), 0);

        // Enqueue some commands
        machine.process_input(CommandInput::Enqueue(vec![1, 2, 3]));
        machine.process_input(CommandInput::Enqueue(vec![4, 5, 6]));

        assert!(!machine.is_empty());
        assert_eq!(machine.pending_count(), 2);

        // Poll in FIFO order
        let out1 = machine.poll_output();
        assert!(matches!(out1, Some(CommandOutput::Command(ref v)) if v == &vec![1, 2, 3]));

        let out2 = machine.poll_output();
        assert!(matches!(out2, Some(CommandOutput::Command(ref v)) if v == &vec![4, 5, 6]));

        // Queue is now empty
        assert!(machine.poll_output().is_none());
        assert!(machine.is_empty());
    }

    #[test]
    fn test_empty_poll_returns_none() {
        let mut machine = CommandQueueMachine::new();
        assert!(machine.poll_output().is_none());
    }
}
