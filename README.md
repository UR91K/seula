# Seula

An Ableton Live project manager. Seula scans your `.als` files, pulls out their metadata
into a local SQLite database, and lets you search, tag and organise them.

Seula is made of three parts:

- **A background service** that runs in the system tray. It owns the database, watches
  your project folders and serves a local HTTP API.
- **A desktop app**, opened from the tray or a shortcut. It is in development; see
  [docs/architecture/frontend.md](docs/architecture/frontend.md).
- **A command-line client**, a small program on your `PATH`, for terminals, scripts and
  AI agents. It talks to the same background service.

The service is the only part that touches your data. The app and the command-line
client both go through it, so you can open, close or call them whenever you like.

Windows-first: macOS and Linux support is planned, paths exist but are currently
untested.

## What it does

- **Parses Ableton Live sets fast.** A single-pass parser runs at about 160–270 MB/s. A
  cold scan of 3,570 projects takes about 90 s, and a warm rescan about 4 s.
- **Extracts per project:** tempo, key and scale, time signature, length, Live version,
  plugins used and samples used.
- **Scans your installed plugins** in a separate worker process, so a crashing plugin
  can't take Seula down. Each project then shows which of its plugins and samples are
  missing on this machine.
- **Full-text search** with operators: `name:` `path:` `plugin:` `sample:` `tag:`
  `collection:` `key:` `bpm:` `ts:` `version:` `dc:` `dm:` (date created / modified).
  Quote a value to keep its spaces: `plugin:"Pro-Q 3"`.
- **Organise projects** with tags, collections (tracklists with cover art), notes, tasks
  and audition audio.
- **Watches** your project folders and picks up changes as they happen.

## Building

Requires a recent stable Rust toolchain.
SQLite is bundled.

```bash
cargo build --release
```

## Running

```bash
seula              # tray mode (default): HTTP API on localhost:50052
seula --server     # the same servers, without the tray icon
seula --help       # CLI
```

On first run, Seula scans your installed plugins before it scans any projects. Expect a
few minutes, because most of that time goes into loading each plugin. After that, a
plugin scan only runs when you ask for one (`seula plugin refresh`).

## Configuration

Seula reads `config.toml` from the first of these locations that exists:

1. the path in the `SEULA_CONFIG` environment variable
2. `%APPDATA%\Seula\config.toml`
3. next to the executable, or in any parent directory

If none exists, it writes a commented default to `%APPDATA%\Seula\config.toml`.

```toml
paths = ['{USER_HOME}/Documents/Ableton Projects']   # folders to scan

# database_path = ''          # default: %APPDATA%\Seula\seula.db
http_port = 50052
log_level = "error"           # error, warn, info, debug, trace
media_storage_dir = '...'     # cover art and audio

# max_cover_art_size_mb = 10
# max_audio_file_size_mb = 50

vst_search_paths = []         # empty = the platform's usual plugin folders
# vst_scan_timeout_secs = 30  # per plugin
```

`{USER_HOME}` expands to your home folder. The environment variables
`SEULA_HTTP_PORT`, `SEULA_LOG_LEVEL` and `SEULA_DATABASE_PATH`
override the matching settings. Restart Seula after you edit the file.

## CLI

```bash
seula scan [PATHS...] [--force]
seula search "plugin:serum bpm:128 key:Am"
seula project list | show | update | delete | restore | rescan | stats
seula plugin  list | search | show | stats | refresh | vendors | formats
seula sample  list | search | stats | check-presence
seula collection | tag | task | system | config ...
seula --cli        # interactive mode
```

Every command takes `--format table|json|csv`. Run `seula <command> --help` for the
details.

> This is an early alpha preview. It works on the database directly, and it will be
> replaced by a separate client that talks to the running service over HTTP, so expect
> command names and flags to change
> ([ADR-0050](docs/decisions/0050-a-cli-client-talks-to-the-daemon-over-http.md)).
> There is no gRPC server any more
> ([ADR-0046](docs/decisions/0046-retire-grpc-and-the-cli-for-a-pure-http-api.md)); the
> HTTP API is the only one.

## API

The HTTP API (axum, `src/http/`) is the supported interface. It's JSON over HTTP, with
Server-Sent Events for scan progress and watcher updates. It accepts requests from
localhost and Tauri origins only. The desktop app and the CLI client are both built on
it.

## AI transparency

Seula was built with AI assistance, alongside a fair amount of manual work.

- **Written by hand:** most of the low-level code, particularly the scanner and
  everything that deals with the `.als` format.
- **Designed by people:** the architecture and design decisions are human ones. AI
  didn't drive the design. It was used for guidance while weighing architectural
  options, and the reasoning behind each decision is recorded in
  [docs/decisions/](docs/decisions/).
- **Where AI helps:** writing tests, looking for bugs, and investigating options for how a feature
  could be implemented before work on it starts.
- **Generated code:** agentic code generation is used often, with an AI agent
  writing the code for a feature. That code is scrutinised and reworked over several
  rounds before it's included.

Every AI contribution is checked thoroughly before it goes in.

## Contributing

Start with [CLAUDE.md](CLAUDE.md) for the commands and the rules that are easy to
break, then [docs/README.md](docs/README.md) for how the docs are organised. Several
choices that look odd are deliberate and written up in
[docs/decisions/](docs/decisions/). Open an issue before you start on anything large.

Contributions are accepted under the licence of the files they change, and nothing
more. There is no contributor licence agreement, so Seula cannot be taken proprietary
by anyone, including its maintainer.

## Licence

Seula is free software: you can redistribute it and/or modify it under the terms of the
GNU Affero General Public License as published by the Free Software Foundation, either
version 3 of the License, or (at your option) any later version. See [LICENSE](LICENSE).

The plugin scanners in `crates/vst-meta/` are under the Mozilla Public License 2.0
instead ([crates/vst-meta/LICENSE](crates/vst-meta/LICENSE)). One additional permission,
for linking with the Steamworks SDK, is in [LICENSE-EXCEPTIONS.md](LICENSE-EXCEPTIONS.md).
The reasoning is in [ADR-0054](docs/decisions/0054-seula-is-agpl-and-the-plugin-scanners-stay-mpl.md).

Versions up to and including commit e697019 were released under the Mozilla Public
License 2.0, and remain available under it.
