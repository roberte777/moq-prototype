use impl_trait_for_tuples::impl_for_tuples;

/// The [`SystemResource`] trait indicates that a type is a resource inherently provided by the
/// system context of the application.
///
/// Provides a single method [`generate`](SystemResource::generate) which takes no input, producing
/// an instance of the resource from only the implicit system context.
///
/// This is intended for usage with [`SystemInput`] in order to define generic handling of system
/// input into a [`StateMachine`](super::StateMachine).
pub trait SystemResource {
    /// Produce an instance of this resource with no direct input, drawing only from the implicitly
    /// available global system context.
    fn generate() -> Self;
}

#[impl_for_tuples(1, 12)]
impl SystemResource for Tuple {
    fn generate() -> Self {
        for_tuples!( ( #( Tuple::generate() ),* ) )
    }
}

impl SystemResource for std::time::Instant {
    fn generate() -> Self {
        std::time::Instant::now()
    }
}

/// A [`StateMachine`](super::StateMachine) input wrapper for providing [`SystemResource`] to the
/// state machine.
pub enum SystemInput<I, S> {
    Input(I),
    System(S),
}
