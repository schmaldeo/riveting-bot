# Riveting Bot

This is, the one and only, Riveting bot.
It's built in [Rust][rust-lang] with [Twilight] and runs in a [Docker][docker] container ...or without.

Primarily made for the **Riveting** community discord server. _(And to keep that one guy in check.)_

# How to build and run

The bot will read the discord token from the environment variable `DISCORD_TOKEN`,
which must be set for the bot to connect.

You may use a `.env` file in the project root directory to specify the token
or any other environment variables for the bot.

## Build with Rust

- Have [rust-lang] installed with latest nightly toolchain.
- _a)_ To just build it, run `cargo build` _(for debug build)_ or `cargo build --release` _(for optimized build)_.
- _b)_ Or to build and run: `cargo run` or `cargo run --release`.
- _(Optional)_ If you want to enable extra features, specify them with `--features` option.
- _(Optional)_ You can disable default features with `--no-default-features` option.
- _(Optional)_ You can run the executable directly, once built. By default, found in `./target/<build>/`.

#### Example

`cargo run --release --no-default-features --features=bulk-delete,owner,voice`

## Build with Docker

(As of right now, this will build the bot with the default features and release build.)

- Have [Docker] installed.

- ### With `docker-compose` _(the easy mode)_

  - To run: `docker compose up -d`. To also rebuild the image before starting, add `--build` to it.
  - To stop the container(s): `docker compose down`.

- ### With base `Dockerfile`

  - To build, run `docker build -t riveting-bot .` in the project root directory.
  - To run the container, run `docker run -d --rm --name riveting-bot --env-file .env riveting-bot`
    (you can use `--env` option instead if you don't have a `.env` file).
    You may want to set up a volume bind with `--mount type=bind,source="$(pwd)"/data,target=/data`.
  - To shutdown the container, run `docker stop riveting-bot`.

# Contributing

Yes.

- The best place to search docs for the many crates of `twilight` is probably their [documentation][twilight-docs].

# Notes

- All of bot's data is located in `./data` folder, which will be created if it doesn't exist yet.
  It will contain logs and configs.
- If there is no `./data/bot.json` config file, a default one will be created on the first run.
  Any manual changes to that file while the bot is running _may_ be lost.
- To control what is logged to a log file, the bot uses `RUST_LOG` environment variable.
  eg. `RUST_LOG=info,twilight=debug,riveting_bot=debug` will set everything on `info` level,
  except for `twilight*` and `riveting_bot` sources, which will be logging `debug` level messages.
- Why `twilight` and not `serenity` or something? Because, yes.
- Please bear in mind that this is still at a very early stage, so there is a bunch of messy stuff going on.

[rust-lang]: https://www.rust-lang.org/
[twilight]: https://twilight.rs/
[twilight-docs]: https://api.twilight.rs/twilight/index.html
[docker]: https://www.docker.com/
