use anyhow::anyhow;
use dialoguer::Input;
use mcping::Connection;

fn main() -> Result<(), anyhow::Error> {
    let server_address = Input::<String>::new()
        .with_prompt("Minecraft server address")
        .interact()?;

    let addr = {
        let mut parts = server_address.split(':');
        let host = parts
            .next()
            .ok_or_else(|| anyhow!("no host in server address"))?;

        // Try and get an ip address from the given host.
        let ip = dns_lookup::lookup_host(host)?
            .pop()
            .ok_or_else(|| anyhow!("unable to perform DNS lookup for given host"))?;

        // If a port exists we want to try and parse it and if not we will
        // default to 25565 (Minecraft).
        let port = if let Some(port) = parts.next() {
            port.parse::<u16>()?
        } else {
            25565
        };

        (ip, port)
    };

    let mut conn = Connection::new(addr).unwrap();
    let (latency, status) = conn.get_status().unwrap();

    println!("version: {}", &status.version.name);
    println!("description: {}", &status.description.text);
    println!(
        "players: {}/{}",
        &status.players.online, &status.players.max
    );
    if let Some(sample) = &status.players.sample {
        print!("sample: ");
        for player in sample.iter().take(sample.len() - 1) {
            print!("{}, ", player.name);
        }

        if !sample.is_empty() {
            print!("{}", &sample.last().unwrap().name);
        }

        println!();
    }
    println!("latency: {}", latency);

    Ok(())
}
