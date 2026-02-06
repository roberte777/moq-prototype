use bon::Builder;

/// Configuration for the RPC router.
#[derive(Debug, Clone, Builder)]
pub struct RpcRouterConfig {
    /// Prefix for client announcements (e.g., "drone").
    /// The router listens for announcements under this prefix.
    /// Defaults to "client" if not specified.
    pub client_prefix: Option<String>,

    /// Prefix for server responses (e.g., "server").
    /// Responses are published at `{response_prefix}/{client_id}/{grpc_path}`.
    /// Defaults to "server" if not specified.
    pub response_prefix: Option<String>,

    /// Track name for RPC messages (e.g., "primary").
    #[builder(default = "primary".to_string())]
    pub track_name: String,
}

impl RpcRouterConfig {
    /// Get the client prefix, defaulting to "client".
    pub fn client_prefix(&self) -> &str {
        self.client_prefix.as_deref().unwrap_or("client")
    }

    /// Get the response prefix, defaulting to "server".
    pub fn response_prefix(&self) -> &str {
        self.response_prefix.as_deref().unwrap_or("server")
    }
}
