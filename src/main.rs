extern crate base64;
extern crate byteorder;
#[macro_use] extern crate error_chain;
extern crate itertools;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate serenity;
extern crate toml;

mod error;
mod ping;

use std::fs::File;
use std::io::prelude::*;

use itertools::Itertools;
use serenity::client::{ Client, Context };
use serenity::prelude::EventHandler;
use serenity::builder::CreateEmbed;
use serenity::model::channel::Message;

struct Handler {
    addr: String,
}

impl EventHandler for Handler {
    fn message(&self, _ctx: Context, msg: Message) {
        if msg.content != "~ping" {
            return;
        }

        let chan = msg.channel_id;

        // Extract the information we want from the response.
        let res = ping::get_response(&self.addr)
            .and_then(|r| {
                // The icon is a base64 encoded PNG so we must decode that first.
                let icon = ping::decode_icon(r.favicon)?;
                // Join the sample player names into a single string.
                let sample = r.players.sample
                    .map(|s| s.into_iter().map(|p| p.name).join(", "))
                    .unwrap_or("None".to_string());

                Ok((icon, r.description, r.players.online, r.players.max, sample))
            });

        match res {
            Ok((icon, desc, online, max, sample)) => {
                // Helper closure to create the basic embed without an icon.
                let basic = |e: CreateEmbed| {
                    e.title(desc)
                        .field("Players", format!("{}/{}", online, max), true)
                        .field("Online", sample, true)
                        .footer(|f| f.text(&self.addr))
                };

                if let Some(icon) = icon {
                    // If there is an icon we will send it as a file. We must first send the file and
                    // then edit the message with an embed because they can't be sent at the same time.
                    let files = [(icon.as_slice(), "icon.png")];
                    chan.send_files(files.iter().cloned(), |m| m)
                        .and_then(|mut m| {
                            // Add the embed to the message that has the file and set the embed thumbnail location.
                            m.edit(|m| {
                                m.embed(|e| {
                                    basic(e)
                                        .thumbnail("attachment://icon.png")
                                })
                            })
                        })
                        .expect("Could not send message.");
                } else {
                    // If there isn't a file being sent we can just send a normal message.
                    chan.send_message(|m| m.embed(basic))
                        .expect("Could not send message.");
                }
            }
            Err(err) => {
                // If there is an error getting the response we will send a message to notify the person
                // requesting the response with the error message.
                chan.send_message(|m| {
                        m.embed(|e| {
                            e.title("Error")
                                .description(&err.to_string())
                        })
                    })
                    .expect("Could not send message.");
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct Config {
    token: String,
    address: String,
}

fn main() {
    let mut path = std::env::current_exe().unwrap();
    path.set_file_name("Config.toml");

    let cfg: Config = {
        let contents = File::open(path)
            .and_then(|mut f| {
                let mut s = String::new();
                f.read_to_string(&mut s)?;
                Ok(s)
            })
            .expect("Could not get config file.");

        toml::from_str(&contents).expect("Could not parse config file.")
    };

    let handler = Handler { addr: cfg.address };
    let mut client = Client::new(&cfg.token, handler).expect("Could not create client.");
    client.start().expect("Could not start client.");
}