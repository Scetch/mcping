mod bedrock;
mod java;

use async_trait::async_trait;

use crate::Error;

/// Represents a pingable entity.
#[async_trait]
pub trait AsyncPingable {
    /// The type of response that is expected in reply to the ping.
    type Response;

    /// Ping the entity, gathering the latency and response.
    async fn ping(self) -> Result<(u64, Self::Response), Error>;
}

/// Retrieve the status of a given Minecraft server using a `AsyncPingable` configuration.
///
///
/// Returns `(latency_ms, response)` where response is a response type of the `Pingable` configuration.
///
/// # Examples
///
/// Ping a Java Server with no timeout:
///
/// ```no_run
/// # async {
/// use std::time::Duration;
///
/// let (latency, response) = mcping::tokio::get_status(mcping::Java {
///     server_address: "mc.hypixel.net".into(),
///     timeout: None,
/// }).await?;
/// # Ok::<(), mcping::Error>(())
/// # };
/// ```
///
/// Ping a Bedrock server with no timeout, trying 3 times:
///
/// ```no_run
/// # async {
/// use std::time::Duration;
///
/// let (latency, response) = mcping::tokio::get_status(mcping::Bedrock {
///     server_address: "play.nethergames.org".into(),
///     timeout: None,
///     tries: 3,
///     ..Default::default()
/// }).await?;
/// # Ok::<(), mcping::Error>(())
/// # };
/// ```
pub async fn get_status<P: AsyncPingable>(pingable: P) -> Result<(u64, P::Response), Error> {
    pingable.ping().await
}
