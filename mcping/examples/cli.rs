use dialoguer::Input;
use mc_legacy_formatting::SpanExt;

fn main() -> Result<(), mcping::Error> {
    let server_address = Input::<String>::new()
        .with_prompt("Minecraft server address")
        .interact()?;

    let (latency, status) = mcping::get_status(&server_address)?;

    print!("version: ");
    status
        .version
        .name
        .span_iter()
        .map(|s| s.wrap_colored())
        .for_each(|s| print!("{}", s));

    println!();
    println!("description:");
    status
        .description
        .text()
        .span_iter()
        .map(|s| s.wrap_colored())
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

    Ok(())
}
