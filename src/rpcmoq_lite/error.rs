use thiserror::Error;

/// Errors that can occur in the RPC-over-MoQ library.
#[derive(Debug, Error)]
pub enum RpcError {
    /// Failed to parse an RPC request path.
    #[error("failed to parse RPC path: {0}")]
    PathParse(String),

    /// A session already exists for this client and RPC path.
    #[error("session already active for client '{client_id}' on '{grpc_path}'")]
    SessionAlreadyActive {
        client_id: String,
        grpc_path: String,
    },

    /// Failed to create a broadcast for the response channel.
    #[error("failed to create broadcast: {0}")]
    BroadcastCreate(String),

    /// Failed to encode a protobuf message.
    #[error("protobuf encode error")]
    Encode(#[from] prost::EncodeError),

    /// Failed to decode a protobuf message.
    #[error("protobuf decode error")]
    Decode(#[from] prost::DecodeError),

    /// An error from the underlying MoQ transport.
    #[error("MoQ transport error")]
    Moq(#[from] moq_lite::Error),

    /// A handler panicked during execution.
    #[error("handler panicked")]
    HandlerPanic,

    /// No handler registered for the given gRPC path.
    #[error("no handler registered for '{0}'")]
    NoHandler(String),

    /// gRPC connection or call failed.
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    /// Timeout waiting for server response broadcast.
    #[error("timeout waiting for server response")]
    Timeout(#[from] tokio::time::error::Elapsed),

    /// Server broadcast not found at expected path.
    #[error("server broadcast not found at path: {0}")]
    ServerNotFound(String),

    /// The RPC connection was closed.
    #[error("RPC connection closed")]
    ConnectionClosed,

    /// Authorization failed for the requested operation.
    #[error("unauthorized: {0}")]
    Unauthorized(String),
}
