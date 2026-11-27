# Contributing

Thanks for looking at this. This file exists so review spends its time on the
idea rather than on the same handful of mechanical things.

## Before you open a PR

Run the gate — it is the same one CI runs:

```
make test                       # cargo test + the panel's contract tests
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo machete                   # no unused dependencies
omarchy plugin validate .       # plugin manifest + entry points
```

`make test` rather than `cargo test`: the panel has Node contract tests
(`omarchy/model.test.mjs`), and a change to the report shape can break them
without touching a line of Rust.

## Checklist

Most review round-trips come from one of these.

- [ ] **`CHANGELOG.md` entry** under `## [Unreleased]`, in the right category
      (`Added` / `Changed` / `Fixed` / `Removed` / `Security`). Anything
      user-visible needs one. **Never edit a released section** — a branch
      older than the last tag carries its entry where `[Unreleased]` used to
      be, and git merges it cleanly into the section that was just published.
      `make changelog-check` proves published sections still match their tags.
- [ ] **Tests that fail without your change.** For a bug fix, confirm the new
      test fails on `main` and passes on your branch.
- [ ] **No leftovers.** If your change removes the last caller of a helper,
      remove the helper too. Dead shared machinery is how this codebase has
      grown copies that drift apart.
- [ ] **Documentation that mentions what you changed.** Grep for it:
      `README.md`, `config.example.toml`, `docs/configuration.md`,
      `docs/vendor-endpoints.md`, `docs/openrouter-accounts.md`.

## Project rules worth knowing

These are in `CLAUDE.md` in full. The ones contributors hit most:

- **`usage` always exits 0** with a valid document. A provider's failure is an
  entry with `status: "error"`, never a failed run — the panel is a display.
- **Tests are hermetic.** A `#[test]` must never read or write a real `$HOME`
  or `$XDG` path, or branch on an ambient environment variable — the AUR
  package runs `cargo test` during install, so a test coupled to your machine
  fails someone else's build. Inject the path: `Cache::at`, not `for_vendor`.
- **Credentials are read, never rewritten** in place. A refreshed token goes to
  the provider's cache, not back into the user's file.
- **One fetch outcome, one fallback policy.** `outcome::Outcome` is the record
  every provider returns and `outcome::fallback` owns what a failed refresh
  means. A guard test fails the build on a second copy.
- **Money and provider names are decided in Rust** — `format::{money, usd}` and
  `VendorId` — and carried in the report. A provider-name or currency table in
  a QML file is a bug.
- **Untrusted text is sanitized at the sink.** A provider's error body is data,
  not a terminal program: `display::sanitize_untrusted_*` runs before it reaches
  the panel.

## Adding a provider

The bar is: **quota reachable with a credential the user already has, obtained
the way this project obtains credentials** — an API key the user holds, or an
OAuth file an official CLI wrote. Scraped browser sessions do not qualify.

Open an issue first with the endpoint, the auth mechanism, and a real response
capture with the numbers redacted. A parser gets pinned to field names and
nesting, and a paraphrase is not enough to build against. Then:

1. A new `src/<provider>/` with `creds`, `types` (wire types + parse, with
   fixture tests) and `fetch` (cache-aware, `Outcome`-returning).
2. A `VendorId` variant plus its `slug`/`display_name`/`short_name`/`config_section`/
   `api_key_env` arms — the matches are exhaustive, so the compiler lists them.
3. A config section, a `provider.rs` fetch arm, and a `sections.rs` projection.
4. `omarchy/Model.js` needs nothing: if the panel needs a new fact, the report
   must carry it.

## Changing the panel's UI

Panel changes are welcome; two rules keep review honest.

- **The panel stays a renderer.** It parses `usage --json` and calls
  `settings show|apply`. Anything else — a provider name table, money
  formatting, a metric order — belongs in Rust.
- **Show a visual change.** Anything that changes how the bar chip, the popup or
  the settings page looks puts before/after screenshots in the PR, for every
  locale and theme it touches. `make omarchy-visual-test` renders the settings
  page offscreen when Quickshell is installed; a look cannot be judged from a
  diff.
