# OpenRouter and DeepSeek account guide

ai-usagebar can report several keys of one provider without running separate
config or cache roots. The provider's own `[<section>]` key is the default
account; `[[<section>.accounts]]` adds named ones. Only OpenRouter and DeepSeek
offer the array — OpenCode Go takes a single key (or the official OpenCode CLI's
own login).

## Add named accounts

One entry per extra key in `~/.config/ai-usagebar/config.toml`:

```toml
[openrouter]
enabled = true
api_key_env = "OPENROUTER_API_KEY"
# api_key = "sk-or-v1-default"

[[openrouter.accounts]]
label = "work"
api_key_env = "OPENROUTER_WORK_API_KEY"

[[openrouter.accounts]]
label = "personal"
api_key = "sk-or-v1-personal"
```

The same shape under `[deepseek]`, with `DEEPSEEK_API_KEY`:

```toml
[deepseek]
enabled = true
[[deepseek.accounts]]
label = "work"
api_key_env = "DEEPSEEK_WORK_API_KEY"
```

An account can use `api_key_env`, an inline `api_key`, or both; the environment
variable wins when both are set. If you store any key inline, ai-usagebar
tightens the config file to mode `0600` on Unix.

Labels cannot be empty, contain path separators, drive prefixes or control
characters, or use a reserved cache filename. Duplicate labels are rejected, and
an entry that names no key source at all is an error rather than a silent
default.

## What one entry separates

Each entry is one login's key — one entry per workspace is the pattern for
working across several workspaces. Be aware of what it does *not* separate: keys
created inside the same OpenRouter workspace share that workspace's billing
account, so two entries holding two keys from one workspace both report that
workspace's credits and spend. The per-entry split is per login session
(workspace), not per key within a single bill.

## Where accounts appear

Named accounts become their own `usage` report entries, with ids
`openrouter@work` and `deepseek@work`, so the Omarchy panel lists one row per
account and switches between them like any other provider.

The default account keeps the provider-root cache
(`~/.cache/ai-usagebar/openrouter/`); a named account uses
`~/.cache/ai-usagebar/openrouter/<label>/`.

To omit the unnamed entry from the report when every key is named:

```toml
[openrouter]
show_default_account = false
```

It stays when `accounts` is empty, so a provider never loses its only entry.

## Recent-model activity (optional management key)

`GET /api/v1/activity` — the "Recent models" block — needs an OpenRouter
*management* key, not the inference key. Set it per account or for the default:

```toml
[openrouter]
management_api_key_env = "OPENROUTER_MANAGEMENT_API_KEY"

[[openrouter.accounts]]
label = "work"
api_key_env = "OPENROUTER_WORK_API_KEY"
management_api_key_env = "OPENROUTER_WORK_MANAGEMENT_API_KEY"
```

The inference key is never sent to `/activity`. A named account never inherits
the default account's management key, and when no management key resolves the
request is skipped rather than fired at a 401 — the rest of the entry still
reports.
