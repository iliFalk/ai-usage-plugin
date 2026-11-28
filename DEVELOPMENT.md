# Development guide

Build and test `ai-usagebar` from a checkout. For the pre-PR gate see
[CONTRIBUTING.md](CONTRIBUTING.md). For the invariants an agent must hold see
[AGENTS.md](AGENTS.md); the long form with the release checklist is
[docs/AGENT-NOTES.md](docs/AGENT-NOTES.md).

- [Prerequisites](#prerequisites)
- [Build](#build)
- [Run locally](#run-locally)
- [Tests](#tests)
- [Lint and format](#lint-and-format)
- [Configuration while developing](#configuration-while-developing)
- [Layout](#layout)

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust / Cargo | **1.90+** (MSRV) | [rustup](https://rustup.rs/) |
| Node.js | 18+ | the Omarchy panel's contract tests (`make plugin-test`) |
| Git | any | |

Optional:

- `cargo-machete` — unused-dependency check (`cargo install cargo-machete`)
- `python3` — `make changelog-check`

## Build

```bash
git clone https://github.com/iliFalk/ai-usage-plugin.git
cd ai-usage-plugin

cargo build --release
```

One binary: `target/release/ai-usagebar`. It is the data source the Omarchy
panel runs; there is no other surface.

Install to `$PREFIX` (default `/usr/local`):

```bash
make install                 # /usr/local/bin + share
make install PREFIX=$HOME/.local
```

## Run locally

From the checkout, without installing:

```bash
./target/release/ai-usagebar usage --json        # the panel's document
./target/release/ai-usagebar usage               # the same data as text
./target/release/ai-usagebar settings show       # what the panel's Settings page reads
```

`--config PATH` points the binary at an alternate TOML, in any argument
position. The file **must already exist**; loads and `settings apply` then read
and write that path for the whole process, so a scratch config never touches the
real one:

```bash
./target/release/ai-usagebar usage --config ./config.test.toml
```

Load the panel itself with Quickshell:

```bash
omarchy plugin validate .
```

## Tests

```bash
make test        # cargo test + the Omarchy panel's contract tests
make plugin-test # node omarchy/model.test.mjs, on its own
```

`cargo test` is hermetic: no test reads or writes a real `$HOME`/`$XDG` path, and
live-API tests stay `#[ignore]`d. The AUR `check()` runs `cargo test` during
`makepkg`, so a test that read a user's customized files would fail the install
on that machine.

## Lint and format

```bash
cargo fmt --all -- --check       # CI fails on a diff
cargo clippy --all-targets -- -D warnings
cargo machete                    # no unused dependencies
omarchy plugin validate .        # plugin manifest + entry points
```

`omarchy-visual-test` renders the real Settings page through Quickshell's
offscreen backend. It is opt-in because a non-Omarchy machine has neither
Quickshell nor the shell's `qs.Commons`/`qs.Ui` modules:

```bash
make omarchy-visual-test
```

## Configuration while developing

`config.example.toml` is the reference: every field with its default. Copy it to
`~/.config/ai-usagebar/config.toml` (or pass `--config`) and uncomment what you
need. An inline `api_key` makes ai-usagebar tighten the file to `0600`.

The panel's Settings page drives `ai-usagebar settings show|apply`. `show` never
prints a secret — inline keys are reported as booleans — and `apply` takes its
patch on stdin so a key never lands in argv or the process environment.

## Layout

| Path | What lives there |
|---|---|
| `src/bin/ai-usagebar.rs` | the binary: argv pre-parse, clap, tokio bootstrap |
| `src/cli.rs` | `usage` and `settings` subcommands |
| `src/provider.rs` | entry list from config, the fetch dispatch, per-entry state |
| `src/sections.rs` | snapshot → ordered rows (the only per-provider metric order) |
| `src/report.rs` | `usage`'s JSON document and text report |
| `src/settings.rs` | `settings show`/`apply`, the only writer of `config.toml` |
| `src/opencode_go/`, `src/openrouter/`, `src/deepseek/` | one provider each: credentials, wire types, fetch, cache |
| `src/{balance,cache,config,countdown,display,error,format,guard,notify,outcome,pacing,usage,vendor}.rs` | shared primitives |
| `omarchy/` | the Omarchy Quattro plugin: bar widget, panel, settings view, report model |
| `docs/` | configuration, provider endpoints, account guide |
