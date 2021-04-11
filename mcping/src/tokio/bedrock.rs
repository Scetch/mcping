//! Implementation of the RakNet ping/pong protocol.
//! https://wiki.vg/Raknet_Protocol#Unconnected_Ping

use async_trait::async_trait;
use std::{
    io::{self, Cursor},
    net::SocketAddr,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::UdpSocket,
};
use trust_dns_resolver::{config::*, TokioAsyncResolver};

use crate::{
    bedrock::{Packet, DEFAULT_PORT, OFFLINE_MESSAGE_DATA_ID},
    tokio::AsyncPingable,
    Bedrock, BedrockResponse, Error,
};

#[async_trait]
impl AsyncPingable for Bedrock {
    type Response = BedrockResponse;

    async fn ping(self) -> Result<(u64, Self::Response), Error> {
        let mut connection =
            Connection::new(&self.server_address, &self.socket_addresses, self.timeout).await?;

        for _ in 0..self.tries {
            connection.send(Packet::UnconnectedPing).await?;

            if let Some(wait) = self.wait_to_try {
                tokio::time::sleep(wait).await;
            }
        }

        let before = Instant::now();
        if let Packet::UnconnectedPong { payload, .. } = connection.read().await? {
            let latency = (Instant::now() - before).as_millis() as u64;

            // Attempt to extract useful information from the payload.
            if let Some(response) = BedrockResponse::extract(&payload) {
                Ok((latency, response))
            } else {
                Err(Error::IoError(io::Error::new(
                    io::ErrorKind::Other,
                    "Invalid Payload",
                )))
            }
        } else {
            Err(Error::IoError(io::Error::new(
                io::ErrorKind::Other,
                "Invalid Packet Response",
            )))
        }
    }
}

/// Extension to `Read` and `ReadBytesExt` that supplies simple methods to write RakNet types.
#[async_trait]
trait AsyncReadBedrockExt: AsyncRead + AsyncReadExt + Unpin {
    /// Writes a Rust `String` in the form Raknet will respond to.
    ///
    /// See more: https://wiki.vg/Raknet_Protocol#Data_types
    async fn read_string(&mut self) -> Result<String, io::Error> {
        let len = self.read_u16().await?;
        let mut buf = vec![0; len as usize];
        self.read_exact(&mut buf).await?;
        String::from_utf8(buf)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Invalid UTF-8 String."))
    }
}

impl<T: AsyncRead + AsyncReadExt + Unpin> AsyncReadBedrockExt for T {}

/// Udp Socket Connection to a Raknet Bedrock Server.
struct Connection {
    socket: UdpSocket,
}

impl Connection {
    async fn new(
        address: &str,
        socket_addresses: &[SocketAddr],
        timeout: Option<Duration>,
    ) -> Result<Self, Error> {
        let mut parts = address.split(':');

        let host = parts.next().ok_or(Error::InvalidAddress)?.to_string();

        let port = if let Some(port) = parts.next() {
            port.parse::<u16>().map_err(|_| Error::InvalidAddress)?
        } else {
            DEFAULT_PORT
        };

        // Do a hostname lookup
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).unwrap();

        let ip = resolver
            .lookup_ip(host.as_str())
            .await
            .ok()
            .and_then(|ips| ips.iter().next())
            .ok_or(Error::DnsLookupFailed)?;

        let socket = UdpSocket::bind(socket_addresses).await?;
        socket.connect((ip, port)).await?;

        let socket = socket.into_std()?;

        socket.set_read_timeout(timeout)?;
        socket.set_write_timeout(timeout)?;

        Ok(Self {
            socket: UdpSocket::from_std(socket)?,
        })
    }

    async fn send(&mut self, packet: Packet) -> Result<(), io::Error> {
        match packet {
            Packet::UnconnectedPing => {
                let mut buf = vec![0x01]; // Packet ID
                buf.write_i64(0x00).await?; // Timestamp
                buf.extend_from_slice(OFFLINE_MESSAGE_DATA_ID); // MAGIC
                buf.write_i64(0).await?; // Client GUID

                self.socket.send(&buf).await?;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Invalid C -> S Packet",
                ))
            }
        }

        Ok(())
    }

    async fn read(&mut self) -> Result<Packet, io::Error> {
        let mut buf = vec![0; 1024];
        self.socket.recv(&mut buf).await?;

        let mut buf = Cursor::new(&buf);

        match buf.read_u8().await? {
            0x1C => {
                // time, server guid, MAGIC, server id
                let time = buf.read_u64().await?;
                let server_id = buf.read_u64().await?;

                let mut tmp = [0; 16];
                buf.read_exact(&mut tmp).await?;

                if tmp != OFFLINE_MESSAGE_DATA_ID {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "incorrect offline message data ID received",
                    ));
                }

                let payload = buf.read_string().await?;

                Ok(Packet::UnconnectedPong {
                    time,
                    server_id,
                    payload,
                })
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                "Invalid S -> C Packet",
            )),
        }
    }
}
