use anyhow::Context as AnyhowContext;
use itertools::Itertools;
use serde::Deserialize;
use serenity::{
    client::{Client, Context},
    http::AttachmentType,
    model::channel::Message,
    prelude::EventHandler,
};
use std::{fs::File, io::prelude::*, time::Duration};

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
    addr: String,
    command: String,
}

impl Handler {
    fn new<S>(addr: S, command: String) -> Result<Self, anyhow::Error>
    where
        S: Into<String>,
    {
        Ok(Handler {
            addr: addr.into(),
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
        let res = mcping::get_status(&self.addr, Duration::from_secs(10)).and_then(|(ping, r)| {
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
                    e.title(desc.text())
                        .fields(vec![
                            ("Players", format!("{}/{}", online, max), true),
                            ("Online", sample, true),
                        ])
                        .footer(|f| f.text(format!("{} | {} ms", &self.addr, ping)));

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
                    m.embed(|e| e.title("Error").description(&err.to_string()))
                })
            }
        };

        // Check if there was an error sending the message.
        if let Err(e) = msg {
            println!("Error sending message: {}", e);
        }
    }
}
