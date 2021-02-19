//! Implementation of the RakNet ping/pong protocol.
//! https://wiki.vg/Raknet_Protocol#Unconnected_Ping

use crate::{Error, Pingable};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{
    io::{self, Cursor, Read},
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, Instant},
};
use trust_dns_resolver::{config::*, Resolver};

/// Raknets default OFFLINE_MESSAGE_DATA_ID.
///
/// See more: https://wiki.vg/Raknet_Protocol#Data_types
const OFFLINE_MESSAGE_DATA_ID: &[u8] = &[
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

/// The default port of a Raknet Bedrock Server.
const DEFAULT_PORT: u16 = 19132;

/// Configuration for pinging a Bedrock server.
///
/// # Examples
///
/// ```
/// use mcping::Bedrock;
/// use std::time::Duration;
///
/// let bedrock_config = Bedrock {
///     server_address: "test.server.com".to_string(),
///     timeout: Some(Duration::from_secs(10)),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Bedrock {
    /// The bedrock server address.
    ///
    /// This can be either an IP or a hostname, and both may optionally have a
    /// port at the end.
    ///
    /// DNS resolution will be performed on hostnames.
    ///
    /// # Examples
    ///
    /// ```text
    /// test.server.com
    /// test.server.com:19384
    /// 13.212.76.209
    /// 13.212.76.209:23193
    /// ```
    pub server_address: String,
    /// The read and write timeouts for the socket.
    pub timeout: Option<Duration>,
    /// The amount of times to try to send the ping packet.
    ///
    /// In case of packet loss an attempt can be made to send more than a single ping.
    pub tries: usize,
    /// The socket addresses to try binding the UDP socket to.
    pub socket_addresses: Vec<SocketAddr>,
}

impl Default for Bedrock {
    fn default() -> Self {
        Self {
            server_address: String::new(),
            timeout: None,
            tries: 5,
            socket_addresses: vec![
                SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 25567)),
                SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 25568)),
                SocketAddr::from((Ipv4Addr::new(0, 0, 0, 0), 25569)),
            ],
        }
    }
}

impl Pingable for Bedrock {
    type Response = BedrockResponse;

    fn ping(self) -> Result<(u64, Self::Response), Error> {
        let mut connection =
            Connection::new(&self.server_address, &self.socket_addresses, self.timeout)?;

        // TODO: don't spam all the packets at once?
        for _ in 0..self.tries {
            connection.send(Packet::UnconnectedPing)?;
        }

        let before = Instant::now();
        if let Packet::UnconnectedPong { payload, .. } = connection.read()? {
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

/// Bedrock Server Payload Response
///
/// See More: https://wiki.vg/Raknet_Protocol#Unconnected_Pong
// TODO: document stuff
#[derive(Clone, Debug)]
pub struct BedrockResponse {
    pub edition: String,
    pub motd_1: String,
    pub protocol_version: String,
    pub version_name: String,
    pub players_online: Option<i64>,
    pub players_max: Option<i64>,
    pub server_id: Option<String>,
    pub motd_2: Option<String>,
    pub game_mode: Option<String>,
    pub game_mode_id: Option<String>,
    pub port_v4: Option<u16>,
    pub port_v6: Option<u16>,
}

impl BedrockResponse {
    /// Extracts information from the semicolon-separated payload.
    ///
    /// Edition (MCPE or MCEE for Education Edition)
    /// MOTD line 1
    /// Protocol Version
    /// Version Name
    /// Player Count
    /// Max Player Count
    /// Server Unique ID
    /// MOTD line 2
    /// Game mode
    /// Game mode (numeric)
    /// Port (IPv4)
    /// Port (IPv6)
    fn extract(payload: &str) -> Option<Self> {
        // TODO: see if it's possible to send ; in a motd line, and figure out
        // how that gets escaped if so
        let mut parts = payload.split(';').map(|s| s.to_string());

        Some(BedrockResponse {
            edition: parts.next()?,
            motd_1: parts.next()?,
            protocol_version: parts.next()?,
            version_name: parts.next()?,
            players_online: parts.next().and_then(|s| s.parse().ok()),
            players_max: parts.next().and_then(|s| s.parse().ok()),
            server_id: parts.next(),
            motd_2: parts.next(),
            game_mode: parts.next(),
            game_mode_id: parts.next(),
            port_v4: parts.next().and_then(|s| s.parse().ok()),
            port_v6: parts.next().and_then(|s| s.parse().ok()),
        })
    }
}

/// Extension to `Read` and `ReadBytesExt` that supplies simple methods to write RakNet types.
trait ReadBedrockExt: Read + ReadBytesExt {
    /// Writes a Rust `String` in the form Raknet will respond to.
    ///
    /// See more: https://wiki.vg/Raknet_Protocol#Data_types
    fn read_string(&mut self) -> Result<String, io::Error> {
        let len = self.read_u16::<BigEndian>()?;
        let mut buf = vec![0; len as usize];
        self.read_exact(&mut buf)?;
        String::from_utf8(buf)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Invalid UTF-8 String."))
    }
}

impl<T: Read + ReadBytesExt> ReadBedrockExt for T {}

/// Represents a RakNet Unconnected Ping Protocol.
#[derive(Debug)]
enum Packet {
    UnconnectedPing,
    UnconnectedPong {
        time: u64,
        server_id: u64,
        payload: String,
    },
}

/// Udp Socket Connection to a Raknet Bedrock Server.
struct Connection {
    socket: UdpSocket,
}

impl Connection {
    fn new(
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
        let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();

        let ip = resolver
            .lookup_ip(host.as_str())
            .ok()
            .and_then(|ips| ips.iter().next())
            .ok_or(Error::DnsLookupFailed)?;

        let socket = UdpSocket::bind(socket_addresses)?;
        socket.connect((ip, port))?;
        socket.set_read_timeout(timeout)?;
        socket.set_write_timeout(timeout)?;

        Ok(Self { socket })
    }

    fn send(&mut self, packet: Packet) -> Result<(), io::Error> {
        match packet {
            Packet::UnconnectedPing => {
                let mut buf = vec![0x01]; // Packet ID
                buf.write_i64::<BigEndian>(0x00)?; // Timestamp
                buf.extend_from_slice(OFFLINE_MESSAGE_DATA_ID); // MAGIC

                // TODO: do we need to get fancy with this or will 0 always be fine?
                buf.write_i64::<BigEndian>(0)?; // Client GUID

                self.socket.send(&buf)?;
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

    fn read(&mut self) -> Result<Packet, io::Error> {
        let mut buf = vec![0; 1024];
        self.socket.recv(&mut buf)?;

        let mut buf = Cursor::new(&buf);

        match buf.read_u8()? {
            0x1C => {
                // time, server guid, MAGIC, server id
                let time = buf.read_u64::<BigEndian>()?;
                let server_id = buf.read_u64::<BigEndian>()?;

                let mut tmp = [0; 16];
                buf.read_exact(&mut tmp)?;

                if tmp != OFFLINE_MESSAGE_DATA_ID {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "incorrect offline message data ID received",
                    ));
                }

                let payload = buf.read_string()?;

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
