/// The output wrapper uses `Result<T, E>` to be able to provide an additional "wait value" when
/// output isn't present.
///
/// Note that state machines still may return `None` if no wait value is applicable.
pub type WrappedOutput<T, E> = Result<T, E>;
