# mcping

[![crate documentation](https://docs.rs/mcping/badge.svg)](https://docs.rs/mcping)
[![Crates.io version](https://img.shields.io/crates/v/mcping.svg)](https://crates.io/crates/mcping)
[![Crates.io downloads](https://img.shields.io/crates/d/mcping.svg)](https://crates.io/crates/mcping)
![CI](https://github.com/Scetch/mcping/workflows/CI/badge.svg)
[![dependency status](https://deps.rs/repo/github/Scetch/mcping/status.svg)](https://deps.rs/repo/github/Scetch/mcping)

`mcping` is a Rust crate that provides Minecraft server ping protocol implementations. It can be used to ping servers and collect information such as the MOTD, max player count, online player sample, server icon, etc.

The library supports both Java and Bedrock servers, and has comprehensive DNS handling (such as SRV record lookup). An async implemention on top of the tokio runtime is also provided.

## Example

Ping a Java Server with no timeout:

```rust
use std::time::Duration;

let (latency, response) = mcping::get_status(mcping::Java {
    server_address: "mc.hypixel.net".into(),
    timeout: None,
})?;
```

Ping a Bedrock server with no timeout, trying 3 times:

```rust
use std::time::Duration;

let (latency, response) = mcping::get_status(mcping::Bedrock {
    server_address: "play.nethergames.org".into(),
    timeout: None,
    tries: 3,
    ..Default::default()
})?;
```

A more complete example can be found in the `cli` example (`examples/cli.rs`) and can be run with `cargo run --example cli`. Some example invocations:

```
cargo run --example cli -- --edition java mc.hypixel.net
cargo run --example cli -- --edition bedrock play.nethergames.org
```

You can run the async version of the example with:

```
cargo run --example cli --features tokio-runtime -- --edition java mc.hypixel.net
cargo run --example cli --features tokio-runtime -- --edition bedrock play.nethergames.org
```

Make sure your working directory is the `mcping` directory when doing so (you can't toggle features from the workspace root).

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
