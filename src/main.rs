extern crate base64;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate byteorder;
#[macro_use] extern crate serenity;
#[macro_use] extern crate lazy_static;

use response::{ ping_response, Response } ;
mod response;

use serenity::client::Client;
use serenity::prelude::EventHandler;
use serenity::framework::standard::StandardFramework;
use serenity::builder::CreateEmbed;

lazy_static! {
    static ref SERVER_ADDR: String = std::env::var("MC_SERVER")
        .expect("MC_SERVER environment variable is not set.");
}

struct Handler;
impl EventHandler for Handler {}

fn main() {
    let token = std::env::var("DISCORD_TOKEN")
        .expect("DISCORD_TOKEN environment variable not set.");
    let mut client = Client::new(&token, Handler)
        .expect("Could not create client.");

    client.with_framework(StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .cmd("ping", ping));

    if let Err(e) = client.start() {
        println!("Error: {:?}", e);
    }
}

command!(ping(_ctx, msg) {
    // We're only going to reply to this if we're in a guild channel.
    let res = ping_response(&SERVER_ADDR)
        .map_err(|e| e.to_string())
        .and_then(|r| {
            serde_json::from_str::<Response>(&r).map_err(|e| e.to_string())
        });

    let chan = msg.channel_id;

    match res {
        Ok(r) => {
            // Serenity does not allow us to send an attachment and embed at the same time so we have 
            // to do this work around for now.
            let icon = {
                r.favicon.as_ref()
                    .map(|icon| {
                        let data = &icon.as_bytes()["data:image/png;base64;".len()..];
                        let content = base64::decode_config(&data, base64::MIME).expect("Invalid icon data.");
                        let files: Vec<(&[u8], &str)> = vec![(&content, "icon.png")];
                        chan.send_files(files, |m| m).expect("Could not send icon file.")
                    })
            };

            let build_embed = |e: CreateEmbed| {
                let online = r.players.sample
                    .map(|sample| {
                        sample.iter()
                            .enumerate()
                            .fold(String::new(), |mut s, (idx, p)| {
                                s.push_str(&p.name);
                                if idx < sample.len() - 1 {
                                    s.push_str(", ");   
                                }
                                s
                            })
                    })
                    .unwrap_or("None".to_string());

                e.title(r.description)
                    .thumbnail("attachment://icon.png")
                    .field("Players", format!("{}/{}", r.players.online, r.players.max), true)
                    .field("Online", online, true)
                    .footer(|f| f.text(SERVER_ADDR.as_str()))
            };

            if let Some(mut m) = icon {
                let _ = m.edit(|m| m.embed(build_embed));
            } else {
                let _ = chan.send_message(|m| m.embed(build_embed));
            }
        }
        Err(err) => {
            let _ = chan.send_message(|m| m.embed(|e| e.title("Error").description(&err.to_string())));
        }
    }
});