use std::net::{ TcpStream, SocketAddr };
use std::io::{ self, Cursor, Read, Write };

use byteorder::{ BigEndian, ReadBytesExt, WriteBytesExt };

/// Adds methods to read Minecraft datatypes to any type
/// that is both Read and ReadBytesExt
trait ReadMinecraftExt: Read + ReadBytesExt {
    fn read_varint(&mut self) -> io::Result<i32> {
        let mut i = 0; 
        let mut j = 0;
        loop {
            let k = self.read_u8()? as i32;
            i |= (k & 0x7F) << j * 7;
            j += 1;
            if j > 5 { panic!("VarInt too big"); }
            if (k & 0x80) != 128 { break; }
        }
        Ok(i)
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
            if (val & 0xFFFFFF80) == 0 {
                self.write_u8(val as u8)?;
                return Ok(());
            }
            self.write_u8((val & 0x7F | 0x80) as u8)?;
            val >>= 7;
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
pub fn ping_response(ip: &str, port: u16) -> io::Result<String> {
    let addr = SocketAddr::new(ip.parse().unwrap(), port);
    let mut s = TcpStream::connect(addr)?;

    let mut buf = Vec::new();

    {
        // Handshake Packet
        buf.write_u8(0x00)?; // ID
        buf.write_varint(4)?; // Protocol Version
        buf.write_string(ip)?; // Host
        buf.write_u16::<BigEndian>(port)?; // Port
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

        Ok(c.read_string()?) // Response
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
    pub sample: Vec<Player>,
}