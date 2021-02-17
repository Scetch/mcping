# mcping

[![crate documentation](https://docs.rs/mcping/badge.svg)](https://docs.rs/mcping)
[![Crates.io version](https://img.shields.io/crates/v/mcping.svg)](https://crates.io/crates/mcping)
[![Crates.io downloads](https://img.shields.io/crates/d/mcping.svg)](https://crates.io/crates/mcping)
![CI](https://github.com/Scetch/mcping/workflows/CI/badge.svg)

`mcping` is a Rust crate that provides Minecraft server ping protocol implementations. It can be used to ping servers and collect information such as the MOTD, max player count, online player sample, server icon, etc.

The library supports both Java edition and Bedrock edition servers, and has comprehensive DNS handling, including SRV records.

## Example

TODO: update the example in the README and talk about bedrock support

```rust
// Ping the server and gather status information and latency.
let (latency, status) = mcping::get_status("mc.hypixel.net", Duration::from_secs(10))?;

println!("latency: {}", latency);
print!("version: {}", status.version.name);
println!("description: {}", status.description.text());
println!("players: {}/{}", status.players.online, status.players.max);
```

A more complete example can be found in the `cli` example (`examples/cli.rs`) and can be run with `cargo run --example cli`. Some example invocations:

```
cargo run --example cli -- --edition java mc.hypixel.net
cargo run --example cli -- --edition bedrock play.nethergames.org
```

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
