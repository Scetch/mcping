use dialoguer::Input;

fn main() -> Result<(), mcping::Error> {
    let server_address = Input::<String>::new()
        .with_prompt("Minecraft server address")
        .interact()?;

    let (latency, status) = mcping::get_status(&server_address)?;

    println!("version: {}", &status.version.name);
    println!("description: {}", &status.description.text());
    println!(
        "players: {}/{}",
        &status.players.online, &status.players.max
    );

    print!("sample: ");

    let sample = status
        .players
        .sample
        .filter(|sample| !sample.is_empty())
        .map(|sample| {
            sample
                .iter()
                .map(|player| player.name.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        })
        .unwrap_or_else(|| "N/A".to_string());

    println!("{}", sample);

    println!("latency: {}", latency);

    Ok(())
}
