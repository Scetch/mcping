use std::net::{ TcpStream, SocketAddr };
use std::io::{ self, Cursor, Read, Write };

use error;
use base64;
use byteorder::{ BigEndian, ReadBytesExt, WriteBytesExt };
use serde_json;

/// Adds methods to read Minecraft datatypes to any type
/// that is both Read and ReadBytesExt
trait ReadMinecraftExt: Read + ReadBytesExt {
    fn read_varint(&mut self) -> io::Result<i32> {
        let mut size = 0;
        let mut res = 0;

        loop {
            let cur = self.read_u8()?;
            let val = (cur & 0b01111111) as i32;
            res |= val << (7 * size);

            size += 1;
            if size > 5 {
                return Err(::std::io::Error::new(::std::io::ErrorKind::Other, "VarInt too big!"));
            }

            if cur & 0b10000000 == 0 {
                break;
            }
        }

        Ok(res)
    }

    fn read_string(&mut self) -> io::Result<String> {
        let len = self.read_varint()? as usize;
        let mut buf = vec![0; len as usize];
        self.read_exact(&mut buf)?;
        Ok(String::from_utf8(buf).expect("Invalid UTF-8 String."))
    }

    fn read_payload(&mut self, mut buf: &mut Vec<u8>) -> io::Result<()> {
        let len = self.read_varint()? as usize;
        buf.resize(len as usize, 0);
        self.read_exact(&mut buf)?;
        Ok(())
    }
}

impl<T> ReadMinecraftExt for T where T: Read + ReadBytesExt {}

/// Adds methods to write Minecraft datatypes to any type
/// that is both Write and WriteBytesExt
trait WriteMinecraftExt: Write + WriteBytesExt {
    fn write_varint(&mut self, mut val: i32) -> io::Result<()> {
        loop {
            let mut tmp = (val & 0b01111111) as u8;
            val >>= 7;
            
            if val != 0 {
                tmp |= 0b10000000;
            }

            self.write_u8(tmp)?;
        
            if val == 0 {
                return Ok(());
            }
        }
    }

    fn write_string(&mut self, s: &str) -> io::Result<()> {
        self.write_varint(s.len() as i32)?;
        self.write_all(s.as_bytes())?;
        Ok(())
    }

    fn write_payload(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write_varint(buf.len() as i32)?;
        self.write_all(buf)?;
        Ok(())       
    }
}

impl<T> WriteMinecraftExt for T where T: Write + WriteBytesExt {}

/// Ping the server and get a response.
pub fn get_response(addr: &str) -> error::Result<Response> {
    let addr: SocketAddr = addr.parse().expect("Invalid ip address.");
    let mut s = TcpStream::connect(addr)?;

    let mut buf = Vec::new();

    {
        // Handshake Packet
        buf.write_u8(0x00)?; // ID
        buf.write_varint(4)?; // Protocol Version
        buf.write_string(&addr.ip().to_string())?; // Host
        buf.write_u16::<BigEndian>(addr.port())?; // Port
        buf.write_varint(1)?; // State
        s.write_payload(&buf)?;
    }

    buf.clear();

    {
        // Ping Packet
        buf.write_u8(0x00)?; // ID
        s.write_payload(&buf)?;   
    }

    buf.clear();

    {
        // Response Packet
        s.read_payload(&mut buf)?;
        let mut c = Cursor::new(&buf);
        let id = c.read_varint()?; // ID

        if id != 0x00 { 
            panic!("Packet ID is not response.");
        }

        let response = c.read_string()?;
        Ok(serde_json::from_str::<Response>(&response)?)
    }
}

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
        Some(s) => {
            Ok(Some(base64::decode_config(&s.as_bytes()["data:image/png;base64;".len()..], base64::MIME)?))
        }
        None => Ok(None),
    }
}