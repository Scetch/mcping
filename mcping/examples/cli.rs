use std::time::Duration;

use argh::FromArgs;
use mc_legacy_formatting::SpanExt;
use mcping::{BedrockResponse, JavaResponse};

#[derive(FromArgs)]
/// Test out pinging servers, Bedrock or Java edition.
struct Args {
    /// the server edition to try and ping
    #[argh(option)]
    edition: Edition,

    /// the server address to ping
    #[argh(positional)]
    address: String,
}

enum Edition {
    Java,
    Bedrock,
}

impl std::str::FromStr for Edition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_ref() {
            "java" => Self::Java,
            "bedrock" => Self::Bedrock,
            _ => return Err("invalid edition".into()),
        })
    }
}

#[cfg(not(feature = "tokio-runtime"))]
fn main() -> Result<(), mcping::Error> {
    let args: Args = argh::from_env();

    match args.edition {
        Edition::Java => {
            let (latency, status) = mcping::get_status(mcping::Java {
                server_address: args.address,
                timeout: Some(Duration::from_secs(5)),
            })?;

            print_java(latency, status);
        }
        Edition::Bedrock => {
            let (latency, status) = mcping::get_status(mcping::Bedrock {
                server_address: args.address,
                timeout: Some(Duration::from_secs(5)),
                ..Default::default()
            })?;

            print_bedrock(latency, status);
        }
    }

    Ok(())
}

#[cfg(feature = "tokio-runtime")]
#[tokio::main]
async fn main() -> Result<(), mcping::Error> {
    let args: Args = argh::from_env();

    match args.edition {
        Edition::Java => {
            let (latency, status) = mcping::tokio::get_status(mcping::Java {
                server_address: args.address,
                timeout: Some(Duration::from_secs(5)),
            })
            .await?;

            print_java(latency, status);
        }
        Edition::Bedrock => {
            let (latency, status) = mcping::tokio::get_status(mcping::Bedrock {
                server_address: args.address,
                timeout: Some(Duration::from_secs(5)),
                ..Default::default()
            })
            .await?;

            print_bedrock(latency, status);
        }
    }

    Ok(())
}

fn print_java(latency: u64, status: JavaResponse) {
    println!();
    print!("version: ");
    status
        .version
        .name
        .span_iter()
        .map(|s| s.wrap_colored())
        .for_each(|s| print!("{}", s));

    println!();
    println!();

    println!("description:");
    status
        .description
        .text()
        .span_iter()
        .map(|s| s.wrap_colored())
        .for_each(|s| print!("{}", s));

    println!();
    println!();
    println!(
        "players: {}/{}",
        &status.players.online, &status.players.max
    );

    print!("sample: ");

    status
        .players
        .sample
        .filter(|sample| !sample.is_empty())
        .map(|sample| {
            println!();

            for player in sample {
                player
                    .name
                    .span_iter()
                    .map(|s| s.wrap_colored())
                    .for_each(|s| print!("{}", s));
                println!();
            }
        })
        .unwrap_or_else(|| println!("N/A"));

    println!("latency: {}ms", latency);
    println!("server icon:\n");

    // The icon is a base64 encoded PNG so we must decode that first.
    if let Some(icon_bytes) = status
        .favicon
        .map(|i| {
            base64::decode_config(
                i.trim_start_matches("data:image/png;base64,"),
                base64::STANDARD,
            )
        })
        .transpose()
        .unwrap_or(None)
    {
        if let Ok(icon_img) =
            image::load_from_memory_with_format(&icon_bytes, image::ImageFormat::Png)
        {
            viuer::print(
                &icon_img,
                &viuer::Config {
                    transparent: true,
                    absolute_offset: false,
                    width: Some(32),
                    ..Default::default()
                },
            )
            .expect("image printing failed");
        }
    }

    println!();
}

fn print_bedrock(latency: u64, status: BedrockResponse) {
    println!();
    println!("version: {}", &status.version_name);
    println!("edition: {}", &status.edition);
    println!("game mode: {}", status.game_mode.as_deref().unwrap_or(""));

    // Some fun facts about MOTDs on bedrock:
    //
    // - so far they seem to exclusively use legacy color codes
    // - the random style has a special impl for periods, they turn into animated
    //   colons that warp up and down rapidly
    // - motd_2 is ignored? client displays "motd_1 - v{version}", where the
    //   appended version text is considered part of motd_1 for color code processing
    // - motd_2 seems to mainly be used to return the server software in use (e.g.
    //   PocketMine-MP)
    // - it looks like trailing whitespace might get trimmed from motd_1 (but not
    //   color codes). Need to confirm
    println!();
    print!("description: ");

    let motd = if !status.version_name.is_empty() {
        format!("{} - v{}", &status.motd_1, &status.version_name)
    } else {
        status.motd_1.clone()
    };

    motd.span_iter()
        .map(|s| s.wrap_colored())
        .for_each(|s| print!("{}", s));

    println!();
    println!();
    println!(
        "players: {}/{}",
        &status.players_online.unwrap_or(0),
        &status.players_max.unwrap_or(0)
    );

    println!("latency: {}ms", latency);

    println!();
}
