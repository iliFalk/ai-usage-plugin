# Provider endpoints

The three providers report through two documented APIs and one undocumented
route on OpenCode's own gateway. Parsers are defensive — an unexpected shape is
a schema error on that entry, never a fabricated zero — and cache the last good
payload so a changed response does not blank the panel.

| Provider | Endpoint | What you see |
|---|---|---|
| **OpenCode Go** | `https://opencode.ai/zen/go/v1/usage` (undocumented; the reading the official CLI's usage view shows) | Rolling 5-hour, weekly and monthly `percent` windows with absolute reset timestamps. The window lengths (5h, 7d) are constants with recorded provenance; the monthly cycle follows the subscription, so it comes from `monthly_cycle_day` or is left unpaced. The key comes from `OPENCODE_GO_API_KEY`, an inline `api_key`, or read-only from the official OpenCode CLI's `auth.json` / OpenCode 2 `opencode.db`. |
| **OpenRouter** | `https://openrouter.ai/api/v1/credits` and `/api/v1/key` (documented), plus `/api/v1/activity` for recent models ([management key](openrouter-accounts.md#recent-model-activity-optional-management-key)) | Balance, today/week/month spend, free vs paid tier, the key's own limit, recent models. A negative balance is debt and is drawn as such. |
| **DeepSeek** | `https://api.deepseek.com/user/balance` (documented) | Balance, granted and topped-up credits, in the account's own currency. |

## Behavior on failure

- Network failures keep the previous data visible, marked stale, and the entry
  carries the HTTP code and message.
- HTTP 429 arms a five-minute backoff: the last good snapshot keeps showing, or
  — with nothing cached inside the stale window — the entry reads "rate limited;
  next attempt in 4m" and no request is made until then.
- A credential error names what is missing and where (`[openrouter]`, or
  `[[deepseek.accounts]]` for a named account).
