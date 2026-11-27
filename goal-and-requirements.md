---
id: goal
type: goal-and-requirements
title: AI Usage — one Omarchy panel for three providers
status: draft
tags:
  - omarchy
  - quota
---

# AI Usage — one Omarchy panel for three providers

## Goal

Show, in one native Omarchy Quattro panel, how much of a paid AI coding plan is
used and when it resets. A user adds the plugin to their bar, the panel reads the
numbers from the `ai-usagebar` binary, and nothing else has to be configured for
the providers whose credentials are already on the machine.

## Problem

Quota lives in a different page per provider, and the pages disagree about what
they call used, remaining, and reset. A developer paying several providers has no
single place that answers "will I run out before the window resets".

## Audience

Omarchy users on a single desktop who pay for AI coding plans directly, run the
binary on the same machine that holds the provider credentials, and want the
numbers on their bar rather than in a browser tab.

## Capabilities

- **One panel, one bar chip.** `omarchy/BarWidget.qml` owns the bar slot,
  `omarchy/Panel.qml` the popup: every configured provider, its windows, reset
  countdowns, balances and severity colours, following the Omarchy theme and
  locale.
- **Three providers.** OpenCode Go (rolling / weekly / monthly quota windows),
  DeepSeek (account balance), and OpenRouter (credit balance, key limits, and
  recent-model activity). A provider can hold several named accounts, each its
  own entry.
- **Provider switching.** The bar shows one provider, one chip per provider, or
  scrolls through them, driven by the plugin's own settings.
- **Native settings.** The panel edits the configuration the binary reads —
  primary provider, per-provider on/off, API keys, hidden metrics, display mode,
  monthly cycle day, and the quota-threshold notification settings — through
  `ai-usagebar settings show|apply`.
- **One data contract.** `ai-usagebar usage --json` is the only thing the panel
  parses; the binary decides what a provider reports, how it is labelled, and what
  a failed fetch means.
- **Honest failure.** A failed refresh keeps the last good numbers marked stale,
  and a provider that cannot be read says why in the panel instead of blanking it.

## Non-goals

- **No other frontends.** The Waybar widget, the tabbed TUI, the macOS and
  Windows system-tray popover, the GNOME Shell extension, the KDE Plasma
  plasmoid, the Linux Mint tray, the macOS menu-bar mirror and its format
  placeholders, and the Claude Code mod are out of scope and not part of this
  product. The Omarchy panel is the only surface.
- **No other providers.** Anthropic/Claude (CLI and admin API), Codex/OpenAI,
  GitHub Copilot, Z.AI, Antigravity, Cursor, Kiro, Kimi, Moonshot, MiniMax, Grok,
  SuperGrok, Grok Bot, Nous Research, Command Code, Cline, Devin, Droid, Muse
  Code, Ollama Cloud, Model Studio, DeepInfra, Novita, Kilo, Lycaon/Lyceum,
  OrcaRouter, Synthetic, user-defined `[[custom]]` providers, and the Claude
  Desktop app integration are out of scope and not part of this product.
- **No Claude Code context scanning.** Local session-transcript reading existed
  for the Anthropic entry; with Anthropic gone it has no consumer.
- **No MCP server, no self-update, no account CLI.** None of them serve the panel.

## Principles

- The binary exits 0 and produces a valid report even when a provider fails: the
  panel is a display, not an error dialog.
- Cache writes are atomic and flock-protected, so a second reader never sees a
  half-written snapshot.
- One fetch outcome, one fallback policy: a failed refresh serves the last good
  payload or reports the error that caused it, never a synthesized message about
  the cache.
- Provider names, money formatting and metric order are decided in Rust and
  carried in the report; the panel formats what it is given.
