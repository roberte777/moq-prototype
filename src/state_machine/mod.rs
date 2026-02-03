pub mod echo;
pub mod wrappers;

/// The [`StateMachine`] trait provides calling semantics and indicates the upholding of invariants
/// that guarantee deterministic behavior.
///
/// # Functionality
/// State machines are expected to operate on defined inputs and outputs. Often there are multiple
/// kinds of input and output a given state machine will operate on and in Rust this can be easily
/// represented with an enum containing the different kinds of input/output.
///
/// However, to actually handle the data of a given variant of the input/output, a method operating
/// on the data of that specific variant is required.
///
/// The implementor of this type _could_ define top level input/output types itself and provide the
/// dispatch functionality on it's own inherent impl. However, separating the grouping and mapping
/// semantics to this trait allows for the state machine to stay focused on the actual logic rather
/// than the calling semantics.
///
/// The type groupings are provided by the associated types of [`Input`](StateMachine::Input) and
/// [`Output`](StateMachine::Output). These will most often be an enum when there are multiple
/// input/output variants, but can be a struct in the base case of a single variant.
///
/// Method dispatch is then defined by the methods [`process_input`](StateMachine::process_input)
/// and [`poll_output`](StateMachine::poll_output), handling mapping input and output respectively.
///
/// # Invariants
/// A [`StateMachine`] must be pure in that it's operation does not depend on any external behavior
/// of the broader system. This provides a guarantee that the behavior of the state machine is
/// fully _deterministic_.
///
/// It has the additional affect of allowing specialized synchronization containers to be built
/// for implementors of this trait by relying on the determinism guarantees, specifically those
/// regarding non-blocking behavior.
///
/// Implementors of this trait *must* uphold all the following invariants.
///
/// ## No Interior Mutability
/// All data *must* be "pure" in that it is either immutable or provides mutability only through
/// `&mut` access. In other words interior mutability is strictly prohibited.
///
/// This means no usage of [`std::cell`] like containers or [`std::sync`] like mutual exclusion
/// mechanisms that provide shared mutability, allowing data modification without an `&mut` bound.
///
/// Shared data via smart pointers like [`Rc`](std::rc::Rc) or [`Arc`](std::sync::Arc) are also
/// prohibited even if the type they wrap doesn't provide interior mutability. This is because the
/// strong and weak reference counts are not pure. Weak versions of these smart pointers are also
/// obviously forbidden as upgrading the weak pointer is impure.
///
/// The only form of allowed shared data is `&'static` references to values that do _not_ provide
/// interior mutability. This would typically be desired in order to refer to a `const` value
/// (e.g. `const VAL: &'static str`).
///
/// ## No IO
/// Performing IO is fundamentally impure as it relies on the external state of the operating system.
///
/// This means the usage of IO via [`std::io`], [`std::net`], or libraries which provide similar
/// access to system IO is strictly forbidden.
///
/// ### No System Time
/// Accessing the system clock is a form of dependency on external state. Two otherwise identical
/// executions of the state machine that read either [`std::time::Instant::now`] or
/// [`std::time::SystemTime`] will diverge based on the precise nanosecond they were executed.
///
/// Therefore, accessing system time from within the state machine logic is strictly forbidden.
///
/// Internal usage of time values and calculations is still allowed, however they must be provided
/// deterministically via input.
///
/// ### No System RNG
/// Generating random numbers via system entropy (e.g. `rand::thread_rng` or `/dev/urandom`) makes
/// the state transition non-reproducible.
///
/// Therefore, obtaining a seed from the system environment during execution is strictly forbidden.
///
/// Internal usage of Pseudo-Random Number Generators (PRNGs) is still allowed, however they must be
/// seeded deterministically via input.
///
/// ## No Concurrency
/// Both system threads such as those provided by [`std::thread`] or "green threads" provided by
/// async runtimes, introduce the possibility of indeterminate outcomes through the execution order
/// of threads.
///
/// Since the driving of threads is controlled by an external runtime, the usage of any threads
/// is strictly forbidden.
///
/// Libraries which provide only _parallelism_ and a guarantee of data-race free, determinate
/// computations such as `rayon` could potentially be allowed but require additional consideration.
/// Out of caution these are also forbidden for the time being.
///
/// ### No Async
/// Rust's async model requires that asynchronous tasks be externally driven. This is achieved via
/// the mechanisms provided in the [`std::task`] module.
///
/// The async driver, referred to as the executor, would imply the external state of the executor
/// affecting the state machine through modification of the [`Context`](`std::task::Context`)
/// provided to asynchronous tasks.
///
/// As such any usage of async is strictly forbidden.
///
/// ## No Blocking
/// The execution of the state machine should be similarly deterministic to that of data.
///
/// This is implicitly covered via the previous rules since in order to block would require either
/// the usage a shared data structures that utilizes interior mutability (e.g.[`std::sync::Mutex`],
/// [`std::sync::Barrier`]), or system IO (e.g. [`std::thread::sleep`]).
///
/// Technically an infinite `loop {}` or long running computations are "pure", but these are still
/// forbidden as they are considered degenerate and would otherwise forbid the state machine from
/// being used in async contexts.
///
/// # Side Effects
/// An exception to the invariants are side effects which do not affect the logic of the state
/// machine nor drive the logic of any other system aside from the affect handler.
///
/// This is to allow things such as metrics and logging to be handled ergonomically when writing
/// state machines.
///
/// It is very important that the logic of the state machine *must not* rely on the outcome of
/// calling into these side effects
///
/// # Handling Time and Randomness via Injection
/// While the state machine itself cannot access system time or entropy, it often needs these
/// concepts  to function. These should be treated as **Inputs** rather than side effects.
///
/// A common pattern is to use a "Runner" or "Container" that wraps the pure state machine. This
/// Runner handles the impure system calls (reading the clock, generating random seeds) and passes
/// them into the state machine via the [`Input`](StateMachine::Input) associated type.
///
/// This allows the State Machine to remain pure and deterministic (e.g. for replay testing), while
/// the Runner manages the connection to the real world.
///
/// See the [`wrappers`][super::wrappers] module for standard patterns on how to implicitly provide
/// time and RNG to your state machine via trait bounds.
///
/// # Limitations
/// With the current implementation the [`StateMachine`] trait can only effectively define "mpsc"
/// or round robin "mpmc" semantics to be dispatched to the state machine.
///
/// This is because producers which provide [`Input`](StateMachine::Input) are implicitly able to
/// define which variant they are encoding through the enum variant they construct. Therefore it is
/// trivial for multiple producers to construct their own individual enum variants and pass them to
/// the state machine for processing.
///
/// Consumers on the other hand have no way to encode which [`Output`](StateMachine::Output) variant
/// they intend to poll for. This is because Rust does not provide access to specify just the enum
/// discriminants and get the data type associated with that tag.
///
/// A custom trait bound on [`Output`](StateMachine::Output) to provide discriminant and tag value
/// mappings for the output enum could be introduced to allow for multiple consumers to poll for
/// individual output variants in the future.
///
/// # Example
/// ```ignore
/// // Inherent impl of the state machine logic
///
/// pub struct SignalInverter {
///     data: i64,
///     is_inverted: bool,
///     
///     pending_data: bool,
///     pending_status: bool,
/// }
///
/// impl SignalInverter {
///     fn process_signal(&mut self, signal: u32) {
///         self.data = self.data.signum() * (signal as i64);
///         self.pending_data = true;
///     }
///
///     fn process_invert(&mut self) {
///         self.data = self.data * -1;
///         self.pending_status = true;
///     }
///
///     fn poll_data(&mut self) -> Option<i64> {
///        self.pending_data.then(self.data)
///     }
///
///     fn poll_status(&mut self) -> Option<bool> {
///         self.pending_status.then(self.status)
///     }
/// }
///
/// // StateMachine trait impl to define top-level input/output types and method dispatch mapping
///
/// pub enum SignalInverterInput {
///     Signal(u32),
///     Invert,
/// }
///
/// pub enum SignalInverterOutput {
///     Data(i64),
///     Status(bool),
/// }
///
/// impl StateMachine for SignalInverter {
///     type Input = SignalInverterInput;
///     type Output = SignalInverterOutput;
///
///     fn process_input(&mut self, input: Self::Input) {
///         match input {
///             SignalInverterInput::Signal(signal) => self.process_signal(signal),
///             SignalInverterInput::Invert => self.process_invert(),
///         }
///     }
///
///     fn poll_output(&mut self) -> Option<Self::Output> {
///         if let Some(output) = self.poll_data().map(SignalInverterOutput::Data) {
///             return Some(output)
///         }
///
///         if let Some(output) = self.poll_status().map(SignalInverterOutput::Status) {
///             return Some(output)
///         }
///
///         None
///     }
/// }
/// ```
pub trait StateMachine {
    /// The type of input that is [processed](StateMachine::process_input) by the state machine.
    ///
    /// This is often an enum containing all the possible variants of input, but can also be a
    /// struct when there is only one input variant.
    type Input;
    /// The type of output that is [polled](StateMachine::poll_output) by the state machine.
    ///
    /// This is often an enum containing all the possible variants of output, but can also be a
    /// struct when there is only one output variant.
    type Output;

    /// Process the provided `input` into the state machine.
    ///
    /// The implementor of this method provides the dispatch mapping from the unified
    /// [`Input`](StateMachine::Input) type of this trait to the corresponding method
    /// of the state machine.
    fn process_input(&mut self, input: Self::Input);

    /// Poll the state machine for output, returning the first available output if present.
    ///
    /// The implementor of this trait provides the dispatch mapping from the polling methods of
    /// the state machine to the unified [`Output`](StateMachine::Output) type of this trait.
    fn poll_output(&mut self) -> Option<Self::Output>;
}
