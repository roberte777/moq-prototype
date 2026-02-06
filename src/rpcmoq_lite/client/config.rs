use std::time::Duration;

use bon::Builder;

/// Configuration for the RPC client.
#[derive(Debug, Clone, Builder)]
pub struct RpcClientConfig {
    /// Unique client identifier.
    pub client_id: String,

    /// Prefix for client broadcasts (e.g., "drone").
    /// Client broadcasts are created at `{client_prefix}/{client_id}/{grpc_path}`.
    /// Defaults to "client" if not specified.
    pub client_prefix: Option<String>,

    /// Prefix for server responses (e.g., "server").
    /// Client subscribes to server responses at `{server_prefix}/{client_id}/{grpc_path}`.
    /// Defaults to "server" if not specified.
    pub server_prefix: Option<String>,

    /// Track name for RPC messages (e.g., "primary").
    #[builder(default = "primary".to_string())]
    pub track_name: String,

    /// Timeout for waiting for server response broadcast.
    #[builder(default = Duration::from_secs(30))]
    pub timeout: Duration,
}

impl RpcClientConfig {
    /// Build the client broadcast path for a given gRPC path.
    pub(crate) fn client_path(&self, grpc_path: &str) -> String {
        let prefix = self.client_prefix.as_deref().unwrap_or("client");
        format!("{}/{}/{}", prefix, self.client_id, grpc_path)
    }

    /// Build the expected server response path for a given gRPC path.
    pub(crate) fn server_path(&self, grpc_path: &str) -> String {
        let prefix = self.server_prefix.as_deref().unwrap_or("server");
        format!("{}/{}/{}", prefix, self.client_id, grpc_path)
    }
}
