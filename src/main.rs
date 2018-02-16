extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate byteorder;
#[macro_use] extern crate serenity;

use response::{ ping_response, Response } ;
mod response;

use serenity::client::Client;
use serenity::prelude::EventHandler;
use serenity::framework::standard::StandardFramework;
use std::env;

const SERVER: &str = "IP";

struct Handler;

impl EventHandler for Handler {}

fn main() {
    let token = env::var("DISCORD_TOKEN").expect("No token set up.");
    let mut client = Client::new(&token, Handler).expect("Could not create client.");

    client.with_framework(StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .cmd("ping", ping));

    if let Err(e) = client.start() {
        println!("Error: {:?}", e);
    }
}

command!(ping(_context, message) {
    let resp_str = match ping_response(SERVER, 25565) {
        Ok(resp) => resp,
        Err(_) => {
            let _ = message.reply("Error pinging server: IO Error");
            return Ok(());
        }
    };

    let resp: Response = match serde_json::from_str(&resp_str) {
        Ok(resp) => resp,
        Err(_) => {
            let _ = message.reply("Error pinging server: Invalid JSON response");
            return Ok(());
        }
    };

    let mut msg = format!("\n**{}**\n{}/{} players\n", resp.description, resp.players.online, resp.players.max);
    for player in &resp.players.sample {
        msg.push_str(&player.name);
        msg.push_str(", ");
    }
    msg.pop();
    msg.pop();

    let _ = message.reply(&msg);
});