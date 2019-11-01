extern crate base64;
extern crate byteorder;
extern crate dns_lookup;
extern crate failure;
#[macro_use] extern crate failure_derive;
extern crate itertools;
extern crate rand;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate serenity;
extern crate toml;

use std::fs::File;
use std::io::prelude::*;
use std::net::IpAddr;

use ping::Connection;
use failure::Error;
use itertools::Itertools;
use serenity::client::{ Client, Context };
use serenity::prelude::EventHandler;
use serenity::builder::CreateEmbed;
use serenity::model::channel::Message;

mod ping;

fn main() {
    let cfg = load_config().expect("Couldn't load config.");
    let handler = Handler::new(cfg.address).expect("Could not create handler.");
    let mut client = Client::new(&cfg.token, handler).expect("Could not create client.");
    client.start().expect("Could not start client.");
}

/// Configuration file with a discord token and server address.
#[derive(Debug, Deserialize)]
struct Config {
    token: String,
    address: String,
}

/// Loads a config file with a discord token and server address.
fn load_config() -> Result<Config, Error> {
    let mut file = File::open("config.toml")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(toml::from_str(&contents)?)
}

struct Handler {
    host: String,
    addr: (IpAddr, u16),
}

impl Handler {
    fn new<S>(host: S) -> Result<Self, Error>
        where S: Into<String>
    {
        // We will keep this host string to display in the embed messages.
        let host = host.into();

        let addr = {
            let mut parts = host.split(':');
            let host = parts.next().expect("Missing configuration host.");

            // Try and get an ip address from the given host.
            let ip = dns_lookup::lookup_host(host)?.pop().unwrap();

            // If a port exists we want to try and parse it and if not we will
            // default to 25565 (Minecraft).
            let port = if let Some(port) = parts.next() {
                port.parse::<u16>()?
            } else {
                25565
            };

            (ip, port)
        };

        Ok(Handler { 
            host: host,
            addr: addr,
        })
    }
}

impl EventHandler for Handler {
    fn message(&self, _ctx: Context, msg: Message) {
        if msg.content != "~ping" { return; }

        let chan = msg.channel_id;

        // Retrieve our response, decode the icon, and build our sample.
        let res = Connection::new(self.addr)
            .and_then(|mut c| c.get_status())
            .and_then(|(ping, r)| {
                // The icon is a base64 encoded PNG so we must decode that first.
                let icon = r.favicon
                    .map(|i| base64::decode_config(i.trim_start_matches("data:image/png;base64;"), base64::MIME))
                    .transpose()?;

                // Join the sample player names into a single string.
                let sample = r.players.sample
                    .map(|s| s.into_iter().map(|p| p.name).join(", "))
                    .unwrap_or("None".to_string());

                Ok((icon, r.description, r.players.online, r.players.max, sample, ping))
            });

        // Attempt to send a message to this channel.
        let msg = match res {
            Ok((icon, desc, online, max, sample, ping)) => {
                // Helper closure to create the basic embed without an icon.
                let basic = |e: CreateEmbed| {
                    e.title(desc.text)
                        .field("Players", format!("{}/{}", online, max), true)
                        .field("Online", sample, true)
                        .footer(|f| f.text(&format!("{} | {} ms", &self.host, ping)))
                };

                if let Some(icon) = icon {
                    // If there is an icon we want to send this message with the icon data.
                    let files = vec![(icon.as_slice(), "icon.png")];
                    chan.send_files(files, |m| m.embed(|e| basic(e).thumbnail("attachment://icon.png")))
                } else {
                    // If there isn't a file being sent we can just send a normal message.
                    chan.send_message(|m| m.embed(basic))
                }
            }
            Err(err) => {
                // If there is an error we will send a message with the error content.
                chan.send_message(|m| m.embed(|e| e.title("Error").description(&err.to_string())))
            }
        };

        // Check if there was an error sending the message.
        if let Err(e) = msg {
            println!("Error sending message: {}", e);
        }
    }
}
