use std::fs::File;
use std::io::prelude::*;
use std::net::IpAddr;

use anyhow::Context as AnyhowContext;
use itertools::Itertools;
use mcping::Connection;
use serenity::client::{Client, Context};
use serenity::http::AttachmentType;
use serenity::model::channel::Message;
use serenity::prelude::EventHandler;

use serde::Deserialize;

fn main() -> Result<(), anyhow::Error> {
    let cfg = load_config().with_context(|| format!("failed to load config"))?;
    let handler = Handler::new(cfg.address, cfg.command)
        .with_context(|| format!("failed to create handler"))?;
    let mut client = Client::new(&cfg.token, handler)
        .with_context(|| format!("failed to create Discord client"))?;
    client
        .start()
        .with_context(|| format!("failed to start Discord client"))?;

    Ok(())
}

/// Configuration file with a discord token and server address.
#[derive(Debug, Deserialize)]
struct Config {
    token: String,
    address: String,
    command: String,
}

/// Loads a config file with a discord token and server address.
fn load_config() -> Result<Config, anyhow::Error> {
    let config_file = "config.toml";

    let mut file = File::open(config_file)
        .with_context(|| format!("failed to open file '{}'", config_file))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("failed to read file '{}'", config_file))?;
    Ok(toml::from_str(&contents)
        .with_context(|| format!("failed to parse TOML loaded from file '{}'", config_file))?)
}

struct Handler {
    host: String,
    addr: (IpAddr, u16),
    command: String,
}

impl Handler {
    fn new<S>(host: S, command: String) -> Result<Self, anyhow::Error>
    where
        S: Into<String>,
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
            host,
            addr,
            command,
        })
    }
}

impl EventHandler for Handler {
    fn message(&self, context: Context, msg: Message) {
        let cmd = msg
            .content
            .split_whitespace()
            .next()
            .filter(|&cmd| cmd == self.command);

        if cmd.is_none() {
            return;
        }

        let chan = msg.channel_id;

        // Retrieve our response, decode the icon, and build our sample.
        let res = Connection::new(self.addr)
            .and_then(|mut c| c.get_status())
            .and_then(|(ping, r)| {
                // The icon is a base64 encoded PNG so we must decode that first.
                let icon = r
                    .favicon
                    .map(|i| {
                        base64::decode_config(
                            i.trim_start_matches("data:image/png;base64,"),
                            base64::STANDARD,
                        )
                    })
                    .transpose()
                    .unwrap_or(None);

                let sanitize = |s: &str| {
                    s.chars().fold(String::with_capacity(s.len()), |mut s, c| {
                        match c {
                            '*' | '_' | '~' | '>' | '`' => {
                                s.push('\\');
                                s.push(c);
                            }
                            _ => s.push(c),
                        }
                        s
                    })
                };

                // Join the sample player names into a single string.
                let sample = r
                    .players
                    .sample
                    .map(|s| s.into_iter().map(|p| sanitize(&p.name)).join(", "))
                    .unwrap_or("None".to_string());

                Ok((
                    icon,
                    r.description,
                    r.players.online,
                    r.players.max,
                    sample,
                    ping,
                ))
            });

        // Attempt to send a message to this channel.
        let msg = match res {
            Ok((icon, desc, online, max, sample, ping)) => chan.send_message(&context.http, |m| {
                m.embed(|e| {
                    e.title(desc.text);
                    e.fields(vec![
                        ("Players", format!("{}/{}", online, max), true),
                        ("Online", sample, true),
                    ]);
                    e.footer(|f| {
                        f.text(format!("{} | {} ms", &self.host, ping));
                        f
                    });

                    if icon.is_some() {
                        e.thumbnail("attachment://icon.png");
                    }

                    e
                });

                if let Some(icon) = icon {
                    m.add_file(AttachmentType::Bytes {
                        data: icon.into(),
                        filename: String::from("icon.png"),
                    });
                }

                m
            }),
            Err(err) => {
                // If there is an error we will send a message with the error content.
                chan.send_message(&context.http, |m| {
                    m.embed(|e| {
                        e.title("Error");
                        e.description(&err.to_string());
                        e
                    });
                    m
                })
            }
        };

        // Check if there was an error sending the message.
        if let Err(e) = msg {
            println!("Error sending message: {}", e);
        }
    }
}
