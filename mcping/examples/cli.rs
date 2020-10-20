use dialoguer::Input;
use mc_legacy_formatting::{PrintSpanColored, SpanIter};

fn main() -> Result<(), mcping::Error> {
    let server_address = Input::<String>::new()
        .with_prompt("Minecraft server address")
        .interact()?;

    let (latency, status) = mcping::get_status(&server_address)?;

    print!("version: ");
    SpanIter::new(&status.version.name)
        .map(PrintSpanColored::from)
        .for_each(|s| print!("{}", s));

    println!();
    println!("description:");
    SpanIter::new(&status.description.text())
        .map(PrintSpanColored::from)
        .for_each(|s| print!("{}", s));

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
                SpanIter::new(&player.name)
                    .map(PrintSpanColored::from)
                    .for_each(|s| print!("  {}", s));
                println!();
            }
        })
        .unwrap_or_else(|| println!("N/A"));

    println!("latency: {}ms", latency);

    Ok(())
}
