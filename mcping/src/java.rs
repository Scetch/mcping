//! Implementation of the Java Minecraft ping protocol.
//! https://wiki.vg/Server_List_Ping

use crate::{Error, Pingable};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::Deserialize;
use std::{
    io::{self, Cursor, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    time::{Duration, Instant},
};
use thiserror::Error;
use trust_dns_resolver::{config::*, Resolver};

/// Configuration for pinging a Java server.
///
/// # Examples
///
/// ```
/// use mcping::Java;
/// use std::time::Duration;
///
/// let bedrock_config = Java {
///     server_address: "mc.hypixel.net".to_string(),
///     timeout: Some(Duration::from_secs(10)),
/// };
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Java {
    /// The java server address.
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
    /// The connection timeout if a connection cannot be made.
    pub timeout: Option<Duration>,
}

impl Pingable for Java {
    type Response = JavaResponse;

    fn ping(self) -> Result<(u64, Self::Response), crate::Error> {
        let mut conn = Connection::new(&self.server_address, self.timeout)?;

        // Handshake
        conn.send_packet(Packet::Handshake {
            version: 47,
            host: conn.host.clone(),
            port: conn.port,
            next_state: 1,
        })?;

        // Request
        conn.send_packet(Packet::Request {})?;

        let resp = match conn.read_packet()? {
            Packet::Response { response } => serde_json::from_str(&response)?,
            _ => return Err(Error::InvalidPacket),
        };

        // Ping Request
        let r = rand::random();
        conn.send_packet(Packet::Ping { payload: r })?;

        let before = Instant::now();
        let ping = match conn.read_packet()? {
            Packet::Pong { payload } if payload == r => {
                (Instant::now() - before).as_millis() as u64
            }
            _ => return Err(Error::InvalidPacket),
        };

        Ok((ping, resp))
    }
}

/// The server status reponse
///
/// More information can be found [here](https://wiki.vg/Server_List_Ping).
#[derive(Deserialize)]
pub struct JavaResponse {
    /// The version of the server.
    pub version: Version,
    /// Information about online players
    pub players: Players,
    /// The description of the server (MOTD).
    pub description: Chat,
    /// The server icon (a Base64-encoded PNG image)
    pub favicon: Option<String>,
}

/// Information about the server's version
#[derive(Deserialize)]
pub struct Version {
    /// The name of the version the server is running
    ///
    /// In practice this comes in a large variety of different formats.
    pub name: String,
    /// See https://wiki.vg/Protocol_version_numbers
    pub protocol: i64,
}

/// An online player of the server.
#[derive(Deserialize)]
pub struct Player {
    /// The name of the player.
    pub name: String,
    /// The player's UUID
    pub id: String,
}

/// The stats for players on the server.
#[derive(Deserialize)]
pub struct Players {
    /// The max amount of players.
    pub max: i64,
    /// The amount of players online.
    pub online: i64,
    /// A preview of which players are online
    ///
    /// In practice servers often don't send this or use it for more advertising
    pub sample: Option<Vec<Player>>,
}

/// This is a partial implemenation of a Minecraft chat component limited to just text
// TODO: Finish this object.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum Chat {
    Text { text: String },
    String(String),
}

impl Chat {
    pub fn text(&self) -> &str {
        match self {
            Chat::Text { text } => text.as_str(),
            Chat::String(s) => s.as_str(),
        }
    }
}

trait ReadJavaExt: Read + ReadBytesExt {
    fn read_varint(&mut self) -> io::Result<i32> {
        let mut res = 0i32;
        for i in 0..5 {
            let part = self.read_u8()?;
            res |= (part as i32 & 0x7F) << (7 * i);
            if part & 0x80 == 0 {
                return Ok(res);
            }
        }
        Err(io::Error::new(io::ErrorKind::Other, "VarInt too big!"))
    }

    fn read_string(&mut self) -> io::Result<String> {
        let len = self.read_varint()? as usize;
        let mut buf = vec![0; len as usize];
        self.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf).expect("Invalid UTF-8 String."))
    }
}

impl<T> ReadJavaExt for T where T: Read + ReadBytesExt {}

trait WriteJavaExt: Write + WriteBytesExt {
    fn write_varint(&mut self, mut val: i32) -> io::Result<()> {
        for _ in 0..5 {
            if val & !0x7F == 0 {
                self.write_u8(val as u8)?;
                return Ok(());
            }
            self.write_u8((val & 0x7F | 0x80) as u8)?;
            val >>= 7;
        }
        Err(io::Error::new(io::ErrorKind::Other, "VarInt too big!"))
    }

    fn write_string(&mut self, s: &str) -> io::Result<()> {
        self.write_varint(s.len() as i32)?;
        self.write_all(s.as_bytes())?;
        Ok(())
    }
}

impl<T> WriteJavaExt for T where T: Write + WriteBytesExt {}

#[derive(Debug, Error)]
#[error("invalid packet response `{packet:?}`")]
pub struct InvalidPacket {
    packet: Packet,
}

#[derive(Debug)]
pub(crate) enum Packet {
    Handshake {
        version: i32,
        host: String,
        port: u16,
        next_state: i32,
    },
    Response {
        response: String,
    },
    Pong {
        payload: u64,
    },
    Request {},
    Ping {
        payload: u64,
    },
}

struct Connection {
    stream: TcpStream,
    host: String,
    port: u16,
}

impl Connection {
    fn new(address: &str, timeout: Option<Duration>) -> Result<Self, Error> {
        // Split the address up into it's parts, saving the host and port for later and converting the
        // potential domain into an ip
        let mut parts = address.split(':');

        let host = parts.next().ok_or(Error::InvalidAddress)?.to_string();

        // If a port exists we want to try and parse it and if not we will
        // default to 25565 (Minecraft)
        let port = if let Some(port) = parts.next() {
            port.parse::<u16>().map_err(|_| Error::InvalidAddress)?
        } else {
            25565
        };

        // Attempt to lookup the ip of the server from an srv record, falling back on the ip from a host
        let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();

        // Determine what host to lookup by doing the following:
        // - Lookup the SRV record for the domain, if it exists perform a lookup of the ip from the target
        //   and grab the port pointed at by the record.
        //
        //   Note: trust_dns_resolver should do a recursive lookup for an ip but it doesn't seem to at
        //   the moment.
        //
        // - If the above failed in any way fall back to the normal ip lookup from the host provided
        //   and use the provided port.
        let lookup_ip =
            |host: &str| -> Option<IpAddr> { resolver.lookup_ip(host).ok()?.into_iter().next() };

        let (ip, port) = resolver
            .srv_lookup(format!("_minecraft._tcp.{}.", &host))
            .ok()
            .and_then(|lookup| {
                let record = lookup.into_iter().next()?;
                let ip = lookup_ip(&record.target().to_string())?;
                Some((ip, record.port()))
            })
            .or_else(|| Some((lookup_ip(&host)?, port)))
            .ok_or(Error::DnsLookupFailed)?;

        let socket_addr = SocketAddr::new(ip, port);

        Ok(Self {
            stream: if let Some(timeout) = timeout {
                TcpStream::connect_timeout(&socket_addr, timeout)?
            } else {
                TcpStream::connect(&socket_addr)?
            },
            host,
            port,
        })
    }

    fn send_packet(&mut self, p: Packet) -> Result<(), Error> {
        let mut buf = Vec::new();
        match p {
            Packet::Handshake {
                version,
                host,
                port,
                next_state,
            } => {
                buf.write_varint(0x00)?;
                buf.write_varint(version)?;
                buf.write_string(&host)?;
                buf.write_u16::<BigEndian>(port)?;
                buf.write_varint(next_state)?;
            }
            Packet::Request {} => {
                buf.write_varint(0x00)?;
            }
            Packet::Ping { payload } => {
                buf.write_varint(0x01)?;
                buf.write_u64::<BigEndian>(payload)?;
            }
            _ => return Err(Error::InvalidPacket),
        }
        self.stream.write_varint(buf.len() as i32)?;
        self.stream.write_all(&buf)?;
        Ok(())
    }

    fn read_packet(&mut self) -> Result<Packet, Error> {
        let len = self.stream.read_varint()?;
        let mut buf = vec![0; len as usize];
        self.stream.read_exact(&mut buf)?;
        let mut c = Cursor::new(buf);

        Ok(match c.read_varint()? {
            0x00 => Packet::Response {
                response: c.read_string()?,
            },
            0x01 => Packet::Pong {
                payload: c.read_u64::<BigEndian>()?,
            },
            _ => return Err(Error::InvalidPacket),
        })
    }
}
