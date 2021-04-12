//! `mcping` is a Rust crate that provides Minecraft server ping protocol
//! implementations. It can be used to ping servers and collect information such
//! as the MOTD, max player count, online player sample, server icon, etc.
//!
//! The library supports both Java and Bedrock servers, and has comprehensive DNS
//! handling (such as SRV record lookup). An async implemention on top of the tokio
//! runtime is also provided.
//!
//! The main API surface is [`get_status`].

#[cfg(feature = "tokio-runtime")]
pub mod tokio;

mod bedrock;
mod java;

pub use bedrock::{Bedrock, BedrockResponse};
pub use java::{Chat, Java, JavaResponse, Player, Players, Version};

/// Errors that can occur when pinging a server.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("an invalid packet configuration was sent")]
    InvalidPacket,
    #[error("an I/O error occurred: {0}")]
    IoError(#[from] std::io::Error),
    #[error("a JSON error occurred: {0}")]
    JsonErr(#[from] serde_json::Error),
    #[error("an invalid address was provided")]
    InvalidAddress,
    #[error("DNS lookup for the host provided failed")]
    DnsLookupFailed,
}

/// Represents a pingable entity.
pub trait Pingable {
    /// The type of response that is expected in reply to the ping.
    type Response;

    /// Ping the entity, gathering the latency and response.
    fn ping(self) -> Result<(u64, Self::Response), Error>;
}

/// Retrieve the status of a given Minecraft server using a `Pingable` configuration.
///
///
/// Returns `(latency_ms, response)` where response is a response type of the `Pingable` configuration.
///
/// # Examples
///
/// Ping a Java Server with no timeout:
///
/// ```no_run
/// use std::time::Duration;
///
/// let (latency, response) = mcping::get_status(mcping::Java {
///     server_address: "mc.hypixel.net".into(),
///     timeout: None,
/// })?;
/// # Ok::<(), mcping::Error>(())
/// ```
///
/// Ping a Bedrock server with no timeout, trying 3 times:
///
/// ```no_run
/// use std::time::Duration;
///
/// let (latency, response) = mcping::get_status(mcping::Bedrock {
///     server_address: "play.nethergames.org".into(),
///     timeout: None,
///     tries: 3,
///     ..Default::default()
/// })?;
/// # Ok::<(), mcping::Error>(())
/// ```
pub fn get_status<P: Pingable>(pingable: P) -> Result<(u64, P::Response), Error> {
    pingable.ping()
}
