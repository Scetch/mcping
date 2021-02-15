# mcping

[![crate documentation](https://docs.rs/mcping/badge.svg)](https://docs.rs/mcping)
[![Crates.io version](https://img.shields.io/crates/v/mcping.svg)](https://crates.io/crates/mcping)
[![Crates.io downloads](https://img.shields.io/crates/d/mcping.svg)](https://crates.io/crates/mcping)
![CI](https://github.com/Scetch/mcping/workflows/CI/badge.svg)

`mcping` is a Rust crate that can ping a Minecraft server and collect ping information such as the MOTD, max player count, player sample, etc.

**Note:** `mcping` currently only supports Minecraft Java edition.

## Example

```rust
// Ping the server and gather status information and latency.
let (latency, status) = mcping::get_status("mc.hypixel.net", Duration::from_secs(10))?;

println!("latency: {}", latency);
print!("version: {}", status.version.name);
println!("description: {}", status.description.text());
println!("players: {}/{}", status.players.online, status.players.max);
```

A more complete example can be found in the `cli` example (`examples/cli.rs`) and can be run with `cargo run --example cli`.

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
