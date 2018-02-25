use std::net::{ TcpStream, SocketAddr };
use std::io::{ self, Cursor, Read, Write };

use error;
use base64;
use byteorder::{ BigEndian, ReadBytesExt, WriteBytesExt };
use serde_json;

trait ReadMinecraftExt: Read + ReadBytesExt {
    fn read_varint(&mut self) -> io::Result<i32> {
        let mut size = 0;
        let mut res = 0;

        loop {
            let cur = self.read_u8()?;
            let val = (cur & 0b01111111) as i32;
            res |= val << (7 * size);
            size += 1;
            if size > 5 { return Err(io::Error::new(io::ErrorKind::Other, "VarInt too big!")); }
            if cur & 0b10000000 == 0 { break; }
        }

        Ok(res)
    }

    fn read_string(&mut self) -> io::Result<String> {
        let len = self.read_varint()? as usize;
        let mut buf = vec![0; len as usize];
        self.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf).expect("Invalid UTF-8 String."))
    }
}

impl<T> ReadMinecraftExt for T where T: Read + ReadBytesExt {}

trait WriteMinecraftExt: Write + WriteBytesExt {
    fn write_varint(&mut self, mut val: i32) -> io::Result<()> {
        loop {
            let mut tmp = (val & 0b01111111) as u8;
            val >>= 7;
            if val != 0 { tmp |= 0b10000000;  }
            self.write_u8(tmp)?;
            if val == 0 { return Ok(()); }
        }
    }

    fn write_string(&mut self, s: &str) -> io::Result<()> {
        self.write_varint(s.len() as i32)?;
        self.write_all(s.as_bytes())?;
        Ok(())
    }
}

impl<T> WriteMinecraftExt for T where T: Write + WriteBytesExt {}

#[derive(Deserialize)]
pub struct Response {
    pub version: Version,
    pub players: Players,
    pub description: String,
    pub favicon: Option<String>,
}

#[derive(Deserialize)]
pub struct Version {
    pub name: String,
    pub protocol: i64,
}

#[derive(Deserialize)]
pub struct Player {
    pub name: String,
    pub id: String,
}

#[derive(Deserialize)]
pub struct Players {
    pub max: i64,
    pub online: i64,
    pub sample: Option<Vec<Player>>,
}

pub fn decode_icon(icon: Option<String>) -> error::Result<Option<Vec<u8>>> {
    match icon {
        Some(s) => Ok(Some(base64::decode_config(&s.as_bytes()["data:image/png;base64;".len()..], base64::MIME)?)),
        None => Ok(None),
    }
}

#[allow(dead_code)] // TODO: Handle Pong and Ping
enum Packet {
    Handshake { version: i32, host: String, port: u16, next_state: i32, },
    Response { response: String, },
    Pong { payload: u64, },
    Request {},
    Ping { payload: u64, },
}

pub struct Connection {
    stream: TcpStream,
    host: String,
    port: u16,
}

impl Connection {
    pub fn new(addr: &str) -> error::Result<Self> {
        let addr = addr.parse::<SocketAddr>().expect("Invalid server address.");
        Ok(Self {
            stream: TcpStream::connect(addr)?,
            host: addr.ip().to_string(),
            port: addr.port(),
        })
    }

    pub fn get_status(&mut self) -> error::Result<Response> {
        let (host, port) = (self.host.clone(), self.port);
        self.send_packet(Packet::Handshake { version: 4, host: host, port: port, next_state: 1 })?;
        self.send_packet(Packet::Request {})?;

        match self.read_packet()? {
            Packet::Response { response } => Ok(serde_json::from_str(&response)?),
            _ => panic!("Invalid response packet."),
        }
    }

    fn send_packet(&mut self, p: Packet) -> io::Result<()> {
        let mut buf = Vec::new();
        match p {
            Packet::Handshake { version, host, port, next_state, } => {
                buf.write_varint(0x00)?;
                buf.write_varint(version)?;
                buf.write_string(&host)?;
                buf.write_u16::<BigEndian>(port)?;
                buf.write_varint(next_state)?;
            }
            Packet::Request { } => {
                buf.write_varint(0x00)?;
            }
            Packet::Ping { payload, } => {
                buf.write_varint(0x01)?;
                buf.write_u64::<BigEndian>(payload)?;
            }
            _ => unimplemented!(),
        }
        self.stream.write_varint(buf.len() as i32)?;
        self.stream.write_all(&buf)?;
        Ok(())
    }

    fn read_packet(&mut self) -> io::Result<Packet> {
        let len = self.stream.read_varint()?;
        let mut buf = vec![0; len as usize];
        self.stream.read_exact(&mut buf)?;
        let mut c = Cursor::new(buf);

        Ok(match c.read_varint()? {
            0x00 => Packet::Response { response: c.read_string()?, },
            0x01 => Packet::Pong { payload: c.read_u64::<BigEndian>()?, },
            _ => unimplemented!(),
        })
    }
}