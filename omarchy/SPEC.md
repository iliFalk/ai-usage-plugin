---
id: omarchy
type: module-design
title: omarchy — the Quattro panel and its settings
status: draft
parent: goal
tags:
  - qml
  - quickshell
---

# omarchy — the Quattro panel and its settings

The plugin's only surface: a bar widget plus the popup it anchors, rendered by
Quickshell inside Omarchy. It is a display frontend and a thin caller of the
binary; it holds no provider knowledge.

## Responsibility

- `BarWidget.qml` — the bar slot. Renders one chip, one chip per provider, or a
  scrolling chip from the report, and owns click/hover routing to the popup.
- `Panel.qml` — the popup: per-provider header, metric list with reset countdowns
  and money, severity colours, provider switching, the OpenCode Go pace chart,
  and the loading / stale / error states.
- `SettingsView.qml` — the settings page: primary provider, per-provider on/off,
  API keys, hidden metrics, display mode, monthly cycle day, and the
  quota-threshold notification section.
- `Model.js` — the whole data layer: parses `ai-usagebar usage --json`, decides
  what the bar and panel draw, builds the `settings apply` patch, and owns the
  formatting rules the panel uses — including the pace chart's deviation
  arithmetic and its fixed ±30 pp scale.
- `I18n.js` / `BrandMark.qml` / `icons/` — translations and provider marks.

## Boundary

The panel talks to the binary through exactly two commands, both launched with
`/usr/bin/env` so a missing binary is reported as such:

- `ai-usagebar usage --json` — the report. The panel never calls a provider
  endpoint, never reads a credential, and never reads `config.toml`.
- `ai-usagebar settings show` / `ai-usagebar settings apply` — the settings
  snapshot and its patch (JSON on stdin).

The pace chart draws the report's `history` and nothing else. For each window
the panel keeps `dev = percent - elapsed * 100` — the deviation, in percentage
points, from an even burn of that window — with `elapsed` taken from the
`start`/`resetsAt` pair Rust ships. Zero is the even-pace line, the two curves
are the cumulative windows in their fixed colours (weekly `#4da3ff`, monthly
`#ffb347`), and the dashed projection runs to the window's close, labelled with
the projected final percentage rather than the deviation. Chart, scale and
label columns are drawn on a Canvas inside `Panel.qml`; the curve math lives in
`Model.js` so `model.test.mjs` can exercise it without a Quickshell runtime.

Everything else is plugin state: the Omarchy settings store is used only for
presentation preferences (show value, brand icons, show all, window choice, show
as used/left, locale, hidden metrics). If the panel needs a fact about a provider,
the report must carry it — a provider-name table in QML is a bug.

`Model.js` stays pure: no file, process or date access. It takes report and
settings text and returns values, which is what `model.test.mjs` exercises
without a Quickshell runtime.

## Invariants

- The panel never crashes on a bad report: an unparsable document, a missing
  binary and a failed exit each render a named state with a next step.
- A stale entry is shown as stale, never as fresh numbers.
- Settings writes go through the binary's validating apply path; the panel does
  not edit TOML itself.
- Locale strings live in `I18n.js` for the five supported languages; a new string
  arrives in all of them or not at all.
- A binary older than the panel may send fewer settings fields: a missing field
  hides its section rather than showing an invented default.
- A provider is charted only if it reports accumulating windows. OpenRouter and
  DeepSeek report a balance with no `resetsAt`, so they get no canvas and no
  filler; the chart is drawn for OpenCode Go alone.
- A thin history is a stated state, not a curve: fewer than two samples in a
  window means the panel says history is still collecting. No synthetic point,
  no flat line at zero.

## Out of scope

The GNOME extension, KDE plasmoid, macOS menu bar, Windows/Linux tray, Linux Mint
tray and Claude Code mod are deleted surfaces, not parallel adapters. See
[goal](goal-and-requirements.md).
