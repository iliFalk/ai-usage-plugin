# Configuration reference

The config file is `~/.config/ai-usagebar/config.toml` (on macOS
`~/Library/Application Support/ai-usagebar/config.toml`, with
`~/.config/ai-usagebar/config.toml` honoured when the canonical file is
absent). Every field is optional; a missing file means the defaults.
[`config.example.toml`](../config.example.toml) carries the same fields with
their defaults written out.

OpenRouter is enabled by default. OpenCode Go and DeepSeek are opt-in — their
sections are `enabled = false` until a key is configured. Turning a provider on
is something the panel's Settings page does for you.

The binary accepts `--config <PATH>` in any position to use an alternate file.
The file must already exist; loads and `settings apply` then read and write that
file for the whole process, so a scratch config never touches the real one:

```bash
ai-usagebar usage --json --config ./config.test.toml
ai-usagebar settings show --config ./config.test.toml
```

## `[ui]`

| Key | Meaning |
|---|---|
| `primary` | Provider the bar shows first and the entry the panel opens on: `opencode-go`, `openrouter` or `deepseek`. Unset → the first enabled provider. |
| `overview_vendors` | Providers to list first, in this order. Enabled providers left off the list follow in the canonical order. Omit the key for every enabled provider. A repeated slug keeps its first occurrence; an empty list is set, and is not the same as omitting the key. `usage --json` includes `overview_vendors` only when this is set, so the panel can order its rows without reading this file. |

## `[notifications]`

Quota-threshold desktop notifications, sent through `notify-send`.

| Key | Meaning |
|---|---|
| `enabled` | Whether a crossing notifies at all. Default `true`. |
| `threshold` | Whole percent 1..=100 at which a window notifies. Default `97`; at `100` only an already exhausted window notifies. |

One notification per crossing. A key re-arms only after usage drops seven
points below the threshold, or when the window's reset moves to a later instant
— so a value hovering at the line does not fire on every refresh. Both fields
are also editable from the panel's Settings page.

## Provider sections

| Section | Key source | What it reports |
|---|---|---|
| `[opencode-go]` | `OPENCODE_GO_API_KEY`, inline `api_key`, or the official OpenCode CLI's own login | Rolling 5-hour, weekly and monthly quota windows |
| `[openrouter]` | `OPENROUTER_API_KEY` or inline `api_key` | Credit balance, the key's own limit, recent-model activity |
| `[deepseek]` | `DEEPSEEK_API_KEY` or inline `api_key` | Account balance, granted and topped-up credits |

Each section accepts `enabled`, `api_key_env` and `api_key`. Provider-specific
fields:

- `[opencode-go]` — `monthly_cycle_day` (1..=28) for exact monthly pacing,
  `auth_paths`, `month_start_ms`, `month_days`, and the three dollar limits
  (`rolling_limit`, `weekly_limit`, `monthly_limit`) the reported percentage
  becomes a spend figure against.
- `[openrouter]` — `management_api_key_env` for the optional management key
  (`/activity` only; see [OpenRouter accounts](openrouter-accounts.md)) and
  `headline` (`percent` or `amount`).
- `[deepseek]` — `display_limit` (tank size in the balance's own currency, used
  to draw a meter) and `headline` (`amount` or `percent`).

OpenRouter and DeepSeek also accept named accounts and `show_default_account`
— see [OpenRouter accounts](openrouter-accounts.md). OpenCode Go takes one key.

### Inline keys

An inline `api_key` is written in plain text in this file. ai-usagebar tightens
the file to mode `0600` on load when it finds one, and refuses to run if it
cannot. An environment variable is still the safer choice, and the panel writes
what you paste into it through the same path.

## Reporting

`ai-usagebar usage` prints a text report; `--json` prints the document the panel
parses:

```json
{
  "schema_version": 1,
  "primary": "openrouter@work",
  "overview_vendors": ["openrouter", "deepseek"],
  "entries": [
    {
      "id": "openrouter",
      "name": "openrouter",
      "display_name": "OpenRouter",
      "short_name": "opr",
      "icon": "󱙺",
      "plan": "OpenRouter",
      "status": "ready",
      "error": null,
      "stale": false,
      "fetched_at": "2026-10-27T18:04:11Z",
      "metrics": [
        {
          "label": "Credit balance",
          "percent": 25,
          "value": "$75.00",
          "detail": "$25.00 of $100.00 used (25%)",
          "headline": "percent",
          "severity": "low",
          "reset_at": null,
          "window_secs": 604800
        }
      ],
      "sections": [ ... ]
    }
  ]
}
```

- `id` is `<provider>` or `<provider>@<account>`.
- `status` is `ready` or `error`; an `error` entry carries its message and no
  rows, and the run still exits 0.
- `stale` marks numbers served from the cache after a failed refresh.
- `metrics` is the gauge rows only; `sections` is every row in draw order,
  typed `metric` / `text` / `block` / `spacer`.
- Optional fields are absent, never null, so an older panel keeps working
  against a newer binary.
- `window_secs` is present only when the provider states the window length
  exactly, which is what lets a frontend pace a row against its reset.

## Settings bridge

The panel edits configuration through the binary, never the file:

- `ai-usagebar settings show` prints a versioned, non-secret snapshot: the
  primary provider and its choices, every provider's on/off switch, which
  credentials are configured (booleans only, never values), `notify_enabled`,
  `notify_threshold` and `monthly_cycle_day`.
- `ai-usagebar settings apply` reads an additive JSON patch **on stdin** (so a
  key never appears in argv), validates every field, and writes through
  `toml_edit`: comments, whitespace and unrelated keys survive, and only what
  the patch names is written.
