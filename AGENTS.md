# AGENTS.md

Working notes for an AI agent in this repository. The full text these rules came from is
`docs/AGENT-NOTES.md`; the commands are in `DEVELOPMENT.md`.

## Product

One surface: the Omarchy Quattro panel in `omarchy/`. Three providers: OpenCode Go, OpenRouter,
DeepSeek. There is no tray, TUI, Waybar module, GNOME extension, KDE plasmoid, macOS menu bar or
Windows app, and adding one back is a product decision, not a refactor.

## Never break

- **`usage` exits 0 with a valid document.** A provider failure is an entry with `status: "error"`,
  never a failed run: the panel is a display, not an error dialog.
- **Cache writes are atomic** (tempfile + persist) under a per-provider `flock`.
- **The report contract is additive.** Optional fields are absent, never null, and `schema_version`
  moves only for an incompatible change.
- **One fetch path.** `provider::refresh_one` owns the cache, the backoff and the threshold hook.
- **The panel stays thin.** Fetching, credentials, canonical names, metrics and reset metadata belong
  in Rust; `VendorId::display_name` and `format::{money, usd}` are the shared sources.
- **Untrusted text is sanitized at the sink.** `display::sanitize_untrusted_{line,path,field}` runs
  before a value reaches the report, a notification or any print.
- **No secrets in tracked files.** Inline keys in `config.toml` are the user's choice; a test fixture
  uses a plain word such as `test-client`.
- **Tests are hermetic.** No `#[test]` reads or writes a real `$HOME`/`$XDG` path or branches on an
  ambient environment variable; inject through the seam (`Cache::at`, `SettingsState::from_config_with`,
  `notify::run_at`, `Config::load_from`).

## Secrets

Never `cat` a file that may hold a key, never a loose `env | grep`, and let `settings show` report the
`inline_configured`/`environment_configured` booleans instead of a value.

## Before a commit

`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `make plugin-test`. Release steps and
the tag rules: `docs/AGENT-NOTES.md` §Release checklist.
