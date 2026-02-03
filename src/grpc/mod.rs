mod server;

pub use server::start_server;

pub use crate::drone_proto::echo_service_client::EchoServiceClient;
