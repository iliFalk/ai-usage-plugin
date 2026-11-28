# ai-usagebar

AI plan usage and balances in a native **Omarchy Quattro panel**: how much of a
paid coding plan is used, what is left in a balance, and when it resets.

Three providers:

| Provider | What you see |
|---|---|
| **OpenCode Go** | Rolling 5-hour, weekly and monthly quota windows, each with its reset countdown and pace against the window |
| **OpenRouter** | Credit balance and spend (today / week / month), the key's own remaining limit, recent models |
| **DeepSeek** | Account balance, granted and topped-up credits |

The panel is the only surface. It is a display frontend: a small
`ai-usagebar` binary does the fetching, caching and formatting, and the panel
renders what it returns. There is no separate bar module, tray app, TUI or
desktop integration to keep in step.

## Install

Two pieces. The binary, from the AUR:

```bash
yay -S ai-usagebar-ilifalk-bin   # prebuilt, both architectures
# or: yay -S ai-usagebar-ilifalk # compiles from source
```

The panel, from this repository:

```bash
omarchy plugin install iliFalk/ai-usage-plugin
```

`omarchy plugin validate .` checks a checkout before you install it. Nix users
can run the binary with `nix run github:iliFalk/ai-usage-plugin`.

## Configure

OpenRouter is on by default once a key is set; OpenCode Go and DeepSeek are
opt-in. The panel's Settings page is the normal way in: it picks the primary
provider, switches providers on and off, stores API keys, hides metrics, sets
the display mode and the monthly cycle day, and edits the quota-notification
threshold. It writes `~/.config/ai-usagebar/config.toml` through the binary, so
comments and unrelated keys survive.

Doing it by hand is the fallback — [`config.example.toml`](config.example.toml)
carries every field with its default:

```toml
[ui]
primary = "openrouter"

[notifications]
enabled = true
threshold = 97

[openrouter]
enabled = true
# api_key_env = "OPENROUTER_API_KEY"

[deepseek]
enabled = true
# api_key_env = "DEEPSEEK_API_KEY"

[opencode-go]
enabled = true
# api_key_env = "OPENCODE_GO_API_KEY"
# monthly_cycle_day = 15
```

OpenRouter and DeepSeek also take named accounts (`[[openrouter.accounts]]`), so
several keys of one provider each get their own row — see
[the account guide](docs/openrouter-accounts.md). OpenCode Go reads its key from
the environment, from an inline `api_key`, or from the official OpenCode CLI's
own login.

## Commands

The panel drives two commands; they are the whole surface.

```bash
ai-usagebar usage            # text report
ai-usagebar usage --json     # the document the panel parses
ai-usagebar settings show    # non-secret settings snapshot (never keys)
ai-usagebar settings apply   # JSON patch on stdin, validated, written atomically
```

`--config PATH` points at an alternate file, in any argument position.

## Behavior worth knowing

- **One data path.** `usage --json` is the only thing the panel parses: provider
  names, money formatting and metric order are decided in Rust and carried in
  the document, so a frontend table can't drift from the binary.
- **Honest failure.** A failed refresh keeps the last good numbers, marked
  stale, with the HTTP status beside them; an unconfigured provider says what is
  missing and where. `usage` still exits 0 with a valid document.
- **Polite to endpoints.** Cache writes are atomic and flock-protected, and a
  provider answering HTTP 429 is left alone for five minutes instead of being
  retried every refresh.
- **Notifications.** One `notify-send` per quota crossing, with hysteresis so a
  value hovering at the line does not fire again, and `[notifications]` controls
  it from the panel.

## Documentation

- [Configuration reference](docs/configuration.md)
- [Provider endpoints](docs/vendor-endpoints.md)
- [OpenRouter and DeepSeek accounts](docs/openrouter-accounts.md)
- [Development guide](DEVELOPMENT.md)
- [Contributing](CONTRIBUTING.md)

## License

MIT. The panel's provider marks derive from [lobe-icons](omarchy/icons/LICENSE-lobe-icons.txt).
