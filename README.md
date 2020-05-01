# mcping

Discord bot written in Rust that pings a Minecraft server with a custom command and displays the status information in chat.

![screenshot](screenshot.png)

## Config

Create a config file called `config.toml` in the root directory of the project with the following:

```
token = ""
address = ""
command = ""
```

Where
- `token` is the discord bot token
- `address` is the Minecraft server address
- `command` is the command that will trigger the ping, for example `~ping` or `~minecraft`

## Running

In order to run the project you'll need [Rust](https://www.rust-lang.org/) installed. Once you have it installed and have created the config file you can run the project with `cargo run --release`.
