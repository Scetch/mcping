use dialoguer::Input;
use mc_legacy_formatting::SpanExt;
use std::time::Duration;

fn main() -> Result<(), mcping::Error> {
    let server_address = Input::<String>::new()
        .with_prompt("Minecraft server address")
        .interact()?;

    let (latency, status) = mcping::get_status(&server_address, Duration::from_secs(10))?;

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
                    resize: true,
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
    Ok(())
}
