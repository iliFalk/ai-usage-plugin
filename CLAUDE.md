# CLAUDE.md

Notes for Claude Code (and humans) about working in this repo. Keep tight: these
are invariants we keep almost-forgetting, not a project tour.

The product is one surface — the Omarchy Quattro panel in `omarchy/` — and three
providers: OpenCode Go, OpenRouter, DeepSeek. There is no tray, TUI, Waybar
module, GNOME extension, KDE plasmoid, macOS menu bar or Windows app, and adding
one back is a product decision, not a refactor.

## Release checklist — must do all of these

When cutting a new version (patch, minor, or major):

1. **Bump every version surface** — `Cargo.toml` `version` and the root Omarchy
   `manifest.json` `version` must match the release tag.
2. **Update `CHANGELOG.md`**:
   - Add a new `## [X.Y.Z] — YYYY-MM-DD` section above the previous one.
   - Categorize by **Added / Changed / Fixed / Removed / Security**
     (Keep-A-Changelog).
   - Update the `[Unreleased]` compare link and add the new release link at the
     bottom.
   - **Prove no published section moved**, before tagging: `make changelog-check`
     diffs every released section against its own tag. It must *compare*, not
     grep for removals — a branch that predates the last tag carries its entry
     where `[Unreleased]` used to be, and git merges it *cleanly* into the
     section that was just published (it happened at v1.6.0, v1.8.0 and
     v1.11.0). `make test` also has a git-free guard for the same accident: no
     changelog entry under two versions, no repeated category heading in one
     section.
3. **Bump `packaging/aur/PKGBUILD`** — `pkgver=X.Y.Z`, `pkgrel=1`, reset
   `sha256sums` to `'SKIP'`.
4. **Bump `packaging/aur/PKGBUILD-bin`** — same `pkgver`, `pkgrel=1`, reset both
   `sha256sums_x86_64` and `sha256sums_aarch64` to `('SKIP' 'SKIP')`.
5. **Regenerate both `.SRCINFO`s NOW, before tagging** — the release workflow's
   `verify-version` job rejects the tag if either `.SRCINFO` still carries the
   old `pkgver` (learned at v0.17.0, which never shipped for exactly this
   reason):
   ```
   cd packaging/aur && makepkg --printsrcinfo > .SRCINFO
   # PKGBUILD-bin must be named PKGBUILD for makepkg — use a scratch dir:
   t=$(mktemp -d) && cp PKGBUILD-bin "$t/PKGBUILD" &&
     (cd "$t" && makepkg --printsrcinfo > .SRCINFO-bin) &&
     cp "$t/.SRCINFO-bin" .SRCINFO-bin && rm -rf "$t"
   ```
   The committed files keep `sha256sums = SKIP`; CI pins the real hashes at
   release time.
6. **Run the gate before tagging**:
   ```
   make changelog-check                        # released sections + version files intact
   make test                                   # cargo test + the panel's Node contract tests
   cargo fmt --all -- --check                  # CI runs this; it is a hard gate
   cargo clippy --all-targets -- -D warnings   # clean
   cargo machete                               # no unused deps
   omarchy plugin validate .                   # plugin manifest + entry points
   ```
   `cargo fmt --all -- --check` is on this list because CI fails the build on a
   diff — it was missing here once and a working commit landed on `main` red
   for nothing but a rustfmt line-wrap.
7. **Commit, tag, push**:
   ```
   git commit -m "vX.Y.Z — …"
   git tag -a vX.Y.Z -m "vX.Y.Z — …"
   git push origin main && git push origin vX.Y.Z
   ```
8. **Wait for CI**: the tag push triggers `.github/workflows/release.yml`, which
   builds the x86_64 and aarch64 tarballs and publishes a GitHub Release.
9. **AUR push is automated** when `AUR_SSH_KEY` is set: `publish-aur` pins the
   real sha256s into both PKGBUILDs, regenerates the `.SRCINFO`s and pushes via
   `KSXGitHub/github-actions-deploy-aur`. The manual fallback, when the secret is
   missing:

   **Pin the real sha256s first**:
   ```
   cd packaging/aur
   curl -sLO https://github.com/akitaonrails/ai-usagebar/archive/refs/tags/vX.Y.Z.tar.gz
   sha256sum vX.Y.Z.tar.gz   # paste into PKGBUILD
   curl -sL https://github.com/akitaonrails/ai-usagebar/releases/download/vX.Y.Z/ai-usagebar-linux-x86_64.tar.gz.sha256
   curl -sL https://github.com/akitaonrails/ai-usagebar/releases/download/vX.Y.Z/ai-usagebar-linux-aarch64.tar.gz.sha256
   ```
   Regenerate the `.SRCINFO`s as in step 5, then push to the separate AUR git
   repos:
   - `~/Projects/aur-ai-usagebar` → `ssh://aur@aur.archlinux.org/ai-usagebar.git`
   - `~/Projects/aur-ai-usagebar-bin` → `ssh://aur@aur.archlinux.org/ai-usagebar-bin.git`

   **Always `git fetch origin && git reset --hard origin/master` in each AUR
   clone first.** A previous session may have pushed an intermediate release that
   your local clone never saw. The clones are throwaway: reset, then overlay the
   canonical `packaging/aur/PKGBUILD*` + regenerated `.SRCINFO*`, commit, push.
10. **Reclaim the build storage.** After the release is verified, run
    `cargo clean`: release gates leave tens of GB of incremental artifacts, and
    multi-arch work compounds it. Also remove scratch worktrees under `/tmp`
    (`git worktree remove --force` + `git worktree prune`) and any `pr-*`
    branches fetched during review (`git branch | grep '^  pr-' | xargs git
    branch -D`). Shared caches (`~/.cargo/registry`, `~/.npm`) are not part of
    this.

**Anything skipping 1–10 is an incomplete release.** Tags are immutable; never
force-move a tag once it is pushed — cut a new patch version instead.

## Hard invariants — never break these

- **`usage` always exits 0 with a valid document.** A provider failure is an
  entry with `status: "error"` and a message, never a failed run: the panel is a
  display, not an error dialog. The one non-zero exit is "nothing was collected",
  which the panel treats as a broken setup.
- **Cache writes are atomic** (tempfile + persist) under a per-provider `flock`,
  so concurrent readers never see a partial snapshot and a second reader cannot
  double-fetch.
- **The report contract is additive.** Optional fields are *absent*, never null,
  and `schema_version` moves only for an incompatible change — a panel one
  release behind must keep working against a newer binary.
- **Tag immutability.** Never `git push --force origin vX.Y.Z` once a release is
  public. The one-time exception in v0.3.0 was a mistake.
- **Untrusted text is sanitized at the sink, not at the call site.** A
  provider's error body and a path carrying an account label are data, not
  terminal programs. `display::sanitize_untrusted_{line,path,field}` runs before
  a value reaches the report, the notification title/body or any
  `println!`/`eprintln!`; `AppError::Io`'s `Display` sanitizes its own path so
  every call site is covered. The exception: a path used as an *argument* must
  stay raw, or the filename breaks.
- **No secrets in tracked files.** Inline API keys in `config.toml` are the
  user's choice (and `chmod 600`ed on load and on save), but **never commit** a
  real key. `.gitignore` covers `.env`, `*.credentials.json`, and `.claude/`.
  Test fixtures use plain words (`test-client`).
- **One fetch outcome, one fallback policy.** `outcome::Outcome<T>` is the
  four-field record every provider returns (`VendorOutcome` is
  `Outcome<VendorSnapshot>`; each provider's `FetchOutcome` is an alias, so
  `outcome.map(VendorSnapshot::Whichever)` is the whole conversion). Build one
  with `Outcome::fresh` off the wire or `Outcome::cached` out of the cache.
  `outcome::fallback` owns what a *failed* refresh means: serve the last good
  payload, or return the error that caused the failure — never a synthesized
  message about the cache, because with no figure on screen the error is the
  whole output. A cached payload that will not parse counts as nothing to show
  and reports the original error. Each provider supplies only a closure that
  parses its own cache format. A guard test forbids a second caller of
  `Cache::fallback_payload`: that function is the entry point to this decision,
  and eighteen private copies of it had already drifted into two generations
  that disagreed for five providers. One synthesized error is sanctioned
  because it tells the user something actionable the underlying failure does not:
  `fresh_payload`'s "rate limited; next attempt in 4m" after a 429 armed the
  backoff. It is a *pre-network refusal*, reached only when there is also no
  payload inside `MAX_STALE` to show instead, so it never replaces a figure with
  a note about the cache. Anything beyond it returns the error that caused it.
- **The panel stays thin.** Provider fetching, credentials, canonical product
  names, metric projection and reset metadata belong in Rust.
  `VendorId::display_name` is the shared label source; a provider-name table in a
  QML file is a bug. `format::{money, usd}` is the shared money source — a
  balance can be negative (an OpenRouter overrun) and the sign belongs outside
  the symbol, so never reach for `format!("${v:.2}")`. Build metrics through
  `SectionBuilder::push_metric`/`push_metric_in_window` so the absolute reset and
  the window length travel with the row; never recreate a per-provider
  metric-order table in `report.rs`.
- **One fetch path.** `provider::refresh_one` is the only thing that fetches: it
  owns the cache, the backoff and the quota-threshold notification hook. A second
  fetch path would give the panel and the notification different numbers.
- **Tests are hermetic.** A `#[test]`/`#[tokio::test]` must never read or write a
  real `$HOME`/`$XDG` path (config, cache, creds, Omarchy theme) or branch on an
  ambient env var — the AUR `check()` runs `cargo test` during `makepkg`, so any
  test that reads a user's *customized* files fails the install on their
  machine. Inject the path/dependency through the seam: `Cache::at` not
  `for_vendor`, `SettingsState::from_config_with` not `from_config`,
  `notify::run_at` not `run`, `Config::load_from` not `load`,
  `sections::project` on a constructed `TabState`. *Carve-out:* a test that
  asserts the path *resolver itself* honors the OS convention may read the env
  var it is testing — that variable is the production input being verified, not
  ambient state the test is incidentally coupled to.

## Secret-discipline rules (learned the hard way)

- **Never `cat` a config file** that could contain `api_key` / `token` / OAuth
  credentials. Use `jq 'keys'` for structure, or
  `grep -v 'api_key\|token\|secret\|password'` for the non-secret lines.
- **Never `env | grep …`** without a tight filter: a loose pattern once matched
  `AWS_*` and `WHATSAPP_*` through a shared substring. Prefer
  `printenv VAR | sed 's|.*|<value-set>|'` per variable.
- **Settings output never carries a key.** `settings show` reports
  `inline_configured` / `environment_configured` booleans; a `KeyInput` value is
  written by `settings apply` and never read back out.

## What lives where

- `src/bin/ai-usagebar.rs` — the binary: argv pre-parse (`--config`), clap, tokio
  bootstrap.
- `src/cli.rs` — `usage` and `settings` subcommands; the entire CLI surface.
- `src/provider.rs` — `TabId`/`TabState`, the entry list from config, the fetch
  dispatch, `refresh_one` (cache + backoff + the notification hook).
- `src/sections.rs` — snapshot → ordered `Section`s with reset metadata; the only
  per-provider metric order.
- `src/report.rs` — `usage`'s JSON document and text report; enumerates and
  formats, adds no provider knowledge.
- `src/settings.rs` — `settings show`/`apply`; the only writer of `config.toml`
  (toml_edit, atomic write, `chmod 600`).
- `src/opencode_go/`, `src/openrouter/`, `src/deepseek/` — one provider each:
  credential resolution, wire types + parse, cache-aware fetch.
- `src/{balance,cache,config,countdown,display,error,format,notify,outcome,
  pacing,usage,vendor}.rs` — shared primitives.
- `src/guard.rs` — structural guards (changelog immutability, AUR checksum
  parity) plus the walking source scanner `cache`/`outcome` guards build on.
- `manifest.json`, `omarchy/` — the Quattro plugin: manifest, bar widget, popup,
  settings view, pure report model (`Model.js`), i18n, provider marks, Node
  contract tests (`model.test.mjs`).
- `packaging/aur/PKGBUILD` — source-build AUR package; `PKGBUILD-bin` — prebuilt
  multi-arch AUR package.
- `.github/workflows/ci.yml` — Linux gate + MSRV + nix; `release.yml` —
  tag-driven release and AUR push.
