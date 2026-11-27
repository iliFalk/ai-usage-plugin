---
id: src
type: module-design
title: src — provider client, report contract, settings CLI
status: draft
parent: goal
tags:
  - rust
  - providers
---

# src — provider client, report contract, settings CLI

The Rust crate behind the Omarchy panel. It owns every fact the panel displays —
which providers exist, what each one reports, how a failed fetch is presented, and
what a configuration edit writes. The panel is a renderer of this crate's JSON and
a caller of its settings commands; it decides nothing about providers.

One binary, `ai-usagebar` (`src/bin/ai-usagebar.rs`), a thin argument splitter over
the library. Its surface is two commands plus `--help`/`--version`:

- `usage --json` (and `usage` as text) — the report described below.
- `settings show` / `settings apply` — the configuration snapshot the panel's
  settings view reads, and the patch it writes back (stdin JSON).

## Responsibility

- Fetch each configured provider's usage over the wire (or read its credential and
  cache), normalize it into one snapshot per provider, and project snapshots into
  the ordered metric rows the panel draws.
- Keep the report contract stable and additive: `schema_version` plus `entries[]`,
  where each entry carries identity, status, stale flag, metrics and optional
  fields. Optional fields are absent, never null, so an older panel keeps working
  against a newer binary.
- Own the failure policy (serve the last good payload, or report the error that
  caused the failure) and the rate-limit backoff.

## Boundary

Inside, by concern:

- **Providers** — `opencode_go/`, `deepseek/`, `openrouter/`. Each owns its
  credential resolution, wire types, fetch, and its own cache format. Nothing else
  knows a provider's endpoint. A provider never formats money or labels its own
  rows: it returns a snapshot.
- **Provider client** — the entry list built from config (one entry per
  provider, one per named account), the fetch dispatch, and the per-entry state
  (`Loading`/`Ready`/`Error`). `refresh_one` is the only thing that fetches: it
  owns the cache, the 429 backoff and the quota-threshold notification hook, so
  the report and a notification can never disagree about a number.
- **Projection** — snapshot → ordered sections and metrics, with absolute reset
  times attached to the row they belong to. The only place per-provider metric
  order exists.
- **Report** — `usage`'s document and text output. Enumerates, projects, formats;
  it adds no provider knowledge.
- **Settings** — the JSON snapshot of editable configuration and the
  stdin-patch apply path (toml_edit, atomic write). It is the only writer of
  `config.toml`, and it validates before writing.
- **Notifications** — `notify`: the pure threshold decision, the persisted
  re-arm state and the `notify-send` sink.
- **Shared primitives** — `config`, `cache`, `outcome`, `error`, `balance`,
  `format`, `display`, `countdown`, `pacing`, `vendor`, `usage`,
  `serde_helpers`.

Forbidden reaches:

- No provider module reads the config file itself; `config.rs` resolves it and
  hands the provider its typed section.
- No provider module writes another provider's cache, and none writes
  `config.toml`.
- The report never fetches directly — it goes through the provider client, so the
  panel and any future reader share one fetch path.
- Frontend concerns stay out: no HTML, no Pango markup, no terminal layout
  outside the report's own text mode.

## Invariants

- `usage` exits 0 with a valid document; a provider's failure becomes an entry
  with `status: "error"` and a message, not a failed run.
- Cache files are written atomically (tempfile + persist) under a per-provider
  `flock`, so concurrent readers never see a partial snapshot.
- Credentials are read, never rewritten in place: a refreshed token goes to the
  provider's cache, not back into the user's credential file.
- Untrusted text (a provider's error body, a path with an account label) is
  sanitized at the sink before it reaches the panel.
- Tests are hermetic: no test reads or writes a real `$HOME`/`$XDG` path, and
  live-API tests stay `#[ignore]`d.

## Out of scope

Nothing is stubbed: a surface or provider that is not listed above does not
exist here. What is in scope, and what is explicitly not, is recorded in
[goal](goal-and-requirements.md).
