//! Common wrappers to provide generic functionality to [`StateMachine`] containers that want
//! to provided injected behavior for common state machine dependencies.
//!
//! This is typically to implicitly provide system resources to state machines that require them
//! to be deterministically provided via input.

pub mod input;
pub mod output;
