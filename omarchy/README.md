# Omarchy Quattro plugin

This is the native Omarchy 4 frontend for ai-usagebar. It runs inside
Quattro's long-lived Quickshell process and uses the shared Omarchy UI kit for
the bar button, keyboard-aware panel, hero, controls, typography, spacing,
colors, borders, and popup placement.

The plugin is deliberately a frontend. It executes fixed `ai-usagebar`
commands; the Rust binary remains the only code that reads or writes
configuration, talks to providers, manages refresh locks, and writes caches.

## Install

The plugin does not install its executable dependency. Install `ai-usagebar`
first, then install this repository as the plugin:

```bash
omarchy pkg aur add ai-usagebar-ilifalk-bin
omarchy plugin add https://github.com/iliFalk/ai-usage-plugin.git --enable
```

To use AI Usage in place of Quattro's default Agents widget, disable the stock
widget:

```bash
omarchy plugin disable omarchy.agents
```

Omarchy clones plugin repositories into `~/.config/omarchy/plugins/`. The root
[manifest](../manifest.json) loads `omarchy/BarWidget.qml`, which owns the bar
button and loads `Panel.qml` inside the same plugin. Update or remove it with
the normal plugin commands:

```bash
omarchy plugin update iljafalk.ai-usage
omarchy plugin remove iljafalk.ai-usage
```

## Controls

- Bar: left-click opens the native Quattro usage panel; middle-click or the
  mouse wheel switches to the next provider the bar is showing. A provider
  hidden from the bar stays in the panel. With **Show all providers** on the
  bar draws one chip per entry, and left-clicking a chip opens the panel on
  the entry that chip stands for (clicking the chip the panel already shows
  closes it).
  While the panel is open the chips it is not showing dim to 45%, so the bar
  says which entry the panel belongs to.
  The exact provider or named account is saved
  in the widget's inline `shell.json` settings and restored after shell reloads
  and sleep/unlock cycles. Right-click is not the settings shortcut.
- Panel: click the gear or press `s` to open the native QML settings page.
  Its **Show usage value in the top bar** toggle switches between the normal
  icon-and-value label and a compact icon-only label without hiding panel or
  tooltip details. Its **Show provider name in the top bar** toggle adds the
  provider's three-letter code in front of that value — the same `short_name`
  the report ships — and is off by default. Its **Color-code usage by
  level** toggle paints bar values, panel meters, and the tooltip green →
  yellow → orange → red from the Omarchy theme as usage climbs, and is off by
  default. Its **Show provider logos** toggle, on by default, draws each
  provider's own mark; off, the bar and the panel hero use the generic robot
  icon of the bar before the marks. Its **Top bar usage window**
  dropdown pins the bar to auto (highest), 5-hour, weekly, or monthly; the
  tooltip and panel hero echo the pinned value while the panel rows keep
  showing every window and alert state still follows the highest percent of the
  metrics not hidden. Its **Show usage as** dropdown reads percentages as what
  is used (the default) or what is left of the same window, on the bar, tooltip,
  hero and panel meters. The form is an accordion of Display, Language, Top bar
  window, Show usage as, Primary provider, Providers and Credentials: one
  section is open at a time and the first is open by default. A folded section
  keeps what you typed, so Save still covers it.
  `h`/`l` or Left/Right switches provider, `j`/`k` or Up/Down scrolls, `r`,
  Enter, or Space refreshes, Tab moves to the neighboring bar panel, and Esc
  closes. Mouse-wheel and touchpad scrolling cover long settings forms faster;
  changes to settings take effect only after the Save button at the bottom.
- Shell: `omarchy-shell shell summon iljafalk.ai-usage '{}'` opens the
  panel and `omarchy-shell shell hide iljafalk.ai-usage` closes it.

The panel keeps the last successful report visible when a refresh fails and
labels it accordingly. Provider-level stale cache responses and hard errors
are shown inline, and unlike the bar's alert state they do not turn that bar
red: only the highest-percent visible window decides whether it is alarming, and a
refresh that yields no report at all still marks it. Absolute reset timestamps
are rendered as live countdowns, so an open panel stays accurate between
network refreshes.

## Settings

Open the panel and select the gear, or press `s`, for the native QML settings
form. It changes the same primary provider and API keys as the terminal
Settings overlay; both write the existing ai-usagebar config in place, preserve
comments and unrelated fields, and retain the platform-specific config path.
Stored key values are never sent to Quattro. The shell receives presence
booleans only, and changed keys travel to the Rust config owner over stdin
rather than argv or the environment. Leave a field blank to keep its current
value, or use its clear button to remove an inline key. Saving a new key also
enables that provider, matching the terminal overlay.

The **Providers** section first chooses which entries appear on the top bar.
It includes custom report entries and individual named accounts. The badge counts
the display switches in this list. All built-ins, including inactive providers,
remain available in the advanced fetching section with their own enabled count.
Display changes take effect immediately and persist as exact entry IDs in the
widget's `hiddenBarEntries` setting in `shell.json`. Hidden entries remain
available in the usage panel and keep refreshing. Hiding every entry leaves a
generic usage icon so the panel and settings remain reachable. In single-entry
mode, a hidden panel selection falls back to the first entry still shown.

To enable an inactive built-in, expand **Advanced: provider fetching**, switch it
on and choose Save; normal credentials are still required. That section retains
the original controls
for enabling or disabling fetching independently from top-bar visibility.

The advanced fetching section lists every known built-in provider with an on/off
switch (the per-vendor `enabled` in `config.toml`). Turning a provider off
removes it from the bar, panel and reports; turning one on takes effect on the
next refresh. The switch only ever names built-in providers, and travels over
the same stdin patch as everything else.

Not every provider has a credential field, and a missing one is not an omission.
Both shipped-key providers in that list are API-key backed: `OPENCODE_GO_API_KEY`
for OpenCode Go, and `OPENROUTER_API_KEY` / `DEEPSEEK_API_KEY` for the other two
named accounts included. Nothing here authenticates through a browser login, so
no login button exists in the settings form.

Existing installations need no migration: `config.toml`, environment-variable
precedence and the binary's own behavior are unchanged. If the plugin is updated
before the `ai-usagebar` package, the form offers the terminal settings fallback
until the binary has the native settings bridge.

The plugin's display-only options remain in `~/.config/omarchy/shell.json` and
can be changed through Omarchy's bar UI or CLI:

```bash
# Show only one entry. Use an id printed by `ai-usagebar usage --json`.
omarchy bar set iljafalk.ai-usage provider openrouter
omarchy bar set iljafalk.ai-usage provider deepseek@work

# Empty means all configured entries, with switching in the panel.
omarchy bar set iljafalk.ai-usage provider ''

# Numeric values need --json so shell.json stores a number.
omarchy bar set iljafalk.ai-usage refreshIntervalSec 300 --json

# Booleans also need --json. The default is true for drop-in compatibility.
omarchy bar set iljafalk.ai-usage showValue false --json

# Opt in to the provider tag (ocg, opr, dsk) in front of the value. Default false.
omarchy bar set iljafalk.ai-usage showProvider true --json

# Show every configured provider's icon and usage at once. The default is false.
omarchy bar set iljafalk.ai-usage showAll true --json

# Color-code bar values, panel meters, and the tooltip by usage level
# (green → yellow → orange → red from the Omarchy theme). The default is false.
omarchy bar set iljafalk.ai-usage colorCodeUsage true --json

# Which quota window the top bar shows: auto (highest, the historical
# default), session (5-hour), weekly (7-day), or monthly. The default is auto.
omarchy bar set iljafalk.ai-usage barWindow session

# How percentages read: used (the default) or left, the share that remains of
# the same window. The bar still picks the most-used window; only the number
# drawn for it changes.
omarchy bar set iljafalk.ai-usage showAs left

# Provider logos in the top bar and panel. Off restores the generic robot icon
# (and the short code on each chip with showAll). The default is true.
omarchy bar set iljafalk.ai-usage brandIcons false --json

# Panel and settings language: auto (follow the system locale, the default),
# en, ru, pt-BR, ko, or es.
omarchy bar set iljafalk.ai-usage uiLocale pt-BR
```

The refresh interval is clamped to 30–3600 seconds. The `provider` setting
prefers an exact entry id; if there is no exact match, a base id such as
`deepseek` selects all accounts for that provider. `showValue`,
`showProvider`, and `showAll` change only the top-bar label; `colorCodeUsage`
also recolors the panel meters and tooltip; `barWindow`
changes the top-bar value and its tooltip/hero echo; none hide report
details or change provider fetching. `barWindow` falls
back to the highest percent (balance/text where a vendor has no metric) when
a vendor lacks the pinned window (a balance-only provider, a weekly-only
response, or no monthly pool), so the bar never goes blank. The one provider
with every window lists all three: OpenCode Go's Rolling (5h), Weekly (7d) and
Monthly rows are chosen by their stated length, so `barWindow session` pins the
rolling window whatever the others read. The alert state always follows the
highest percent of the metrics not hidden, never the pinned window.

The **Metrics** section of the panel Settings chooses what each provider shows:
it expands per provider, with one switch per metric. A metric switched off
disappears from the panel, the top bar and the tooltip. The choice is saved as
`hiddenMetrics` in the widget's `shell.json`
settings, a map from an entry id to the metric labels switched off
(`{"opencode-go": ["Monthly"]}`; a metric under a heading is keyed
`Heading / Label`), so each account is independent. A hidden metric is ignored
when the bar picks the highest percentage and when it decides whether the icon
is alarming, so an OpenCode Go entry with Rolling and Weekly almost empty and
the Monthly window switched off stays low however full that window is. The last
metric still on cannot be switched off: its switch is disabled, so the bar never
goes blank (a `hiddenMetrics` that would hide every metric, edited by hand, is
ignored). A balance-only entry (DeepSeek, OpenRouter without credits) has no
metric rows to offer, so its section stays hidden.
`showAs` changes only the number: Left shows what remains of the most-used
window (OpenCode Go's 18% weekly window reads `82%`), and the alert state keeps
following the used share.

`showProvider` draws the `short_name` the Rust report ships for the selected
entry, so the codes are the same ones the report prints: `ocg 42%`,
`opr 25%`, `dsk $8.42`. Every account of one provider shares that provider's
code — the panel and tooltip remain the place that tells `OpenRouter · work`
from `OpenRouter · personal`. With both toggles on the bar reads icon + `ocg 42%`; with
`showValue` off it is the icon and `ocg`. `showAll` draws every visible
entry as its own chip with a brand SVG (see [`icons/README.md`](icons/README.md)
for source and licence) — and each chip is a target for its entry across the
whole bar height and its share of the gaps beside it, so a left-click anywhere
in that column opens the panel there. The three shipped providers each have
their own mark; anything else falls back to the nerd-font glyph from the report.
A vertical bar has room for none of this and keeps showing a single icon.
Against an `ai-usagebar` older than the `short_name` field the tag falls
back to the entry id's provider half (`opencode-go 42%`) until the binary is
updated.

## Development checks

On an Omarchy 4 machine:

```bash
omarchy plugin validate .
node omarchy/model.test.mjs
make omarchy-visual-test
```

The visual test renders the real settings component in dark and light fixture
themes through Quickshell's offscreen backend. It does not open a window or
read credentials. Review the generated PNGs under
`target/omarchy-visual-scenarios/`; they are local test artifacts, not golden
pixel snapshots.

`qmllint` cannot resolve the `qs.*` modules that Omarchy injects at shell
runtime, so it is not a reliable standalone check for plugin entry points.

Saving files under an installed user plugin triggers Quattro's plugin hot
reload. If the running shell keeps the old panel after a change, use
`omarchy restart shell` to reload its QML. In a source checkout, rerun
`omarchy plugin validate .` after changing the manifest or entry points.
