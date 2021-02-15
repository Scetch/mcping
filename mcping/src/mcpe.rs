#![allow(dead_code)]

//! Implementation of the RakNet ping/pong protocol.
//! https://wiki.vg/Raknet_Protocol#Unconnected_Ping

use crate::Error;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{
    io::{self, Cursor, Read},
    net::UdpSocket,
    time::{Duration, Instant},
};
use trust_dns_resolver::{config::*, Resolver};

const MAGIC: &[u8] = &[
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

trait ReadBedrockExt: Read + ReadBytesExt {
    fn read_string(&mut self) -> Result<String, io::Error> {
        let len = self.read_u16::<BigEndian>()?;
        let mut buf = vec![0; len as usize];
        self.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf).expect("Invalid UTF-8 String."))
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

struct Connection {
    socket: UdpSocket,
}

impl Connection {
    fn new(address: &str, timeout: Option<Duration>) -> Result<Self, Error> {
        let mut parts = address.split(':');

        let host = parts.next().ok_or(Error::InvalidAddress)?.to_string();

        let port = if let Some(port) = parts.next() {
            port.parse::<u16>().map_err(|_| Error::InvalidAddress)?
        } else {
            19132
        };

        // Do a hostname lookup
        let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();

        let ip = resolver
            .lookup_ip(host.as_str())
            .ok()
            .and_then(|ips| ips.iter().next())
            .ok_or(Error::DnsLookupFailed)?;

        let socket = UdpSocket::bind("0.0.0.0:25567")?;
        socket.connect((ip, port))?;
        socket.set_read_timeout(timeout)?;
        socket.set_write_timeout(timeout)?;

        Ok(Self { socket })
    }

    fn send(&mut self, packet: Packet) -> Result<(), io::Error> {
        match packet {
            Packet::UnconnectedPing => {
                // id, time, MAGIC, client guid
                let mut buf = vec![0x01]; // We will write the first packets id and timestamp (0)
                buf.write_i64::<BigEndian>(0x00)?;
                buf.extend_from_slice(MAGIC);
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

                if tmp != MAGIC {
                    return Err(io::Error::new(io::ErrorKind::Other, "Magic Mistmatch"));
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
#[derive(Debug)]
pub struct Response {
    edition: String,
    motd_1: String,
    protocol_version: String,
    version_name: String,
    online: Option<i64>,
    max: Option<i64>,
    server_id: Option<String>,
    game_mode: Option<String>,
    game_mode_id: Option<String>,
    port_v4: Option<u16>,
    port_v6: Option<u16>,
}

impl Response {
    /// Extracts information from the semi-colon separated payload.
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
        let mut parts = payload.split(';').map(|s| s.to_string());
        Some(Response {
            edition: parts.next()?,
            motd_1: parts.next()?,
            protocol_version: parts.next()?,
            version_name: parts.next()?,
            online: parts.next().and_then(|s| s.parse().ok()),
            max: parts.next().and_then(|s| s.parse().ok()),
            server_id: parts.next(),
            game_mode: parts.next(),
            game_mode_id: parts.next(),
            port_v4: parts.next().and_then(|s| s.parse().ok()),
            port_v6: parts.next().and_then(|s| s.parse().ok()),
        })
    }
}

pub fn get_status(
    address: &str,
    timeout: impl Into<Option<Duration>>,
) -> Result<(u64, Response), Error> {
    let mut connection = Connection::new(address, timeout.into())?;
    connection.send(Packet::UnconnectedPing)?;

    let before = Instant::now();
    if let Packet::UnconnectedPong { payload, .. } = connection.read()? {
        let latency = (Instant::now() - before).as_millis() as u64;

        // Attempt to extract useful information from the payload.
        if let Some(response) = Response::extract(&payload) {
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