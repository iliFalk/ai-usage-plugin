//! Config file at `~/.config/ai-usagebar/config.toml`.
//!
//! Layout:
//! ```toml
//! [ui]
//! primary = "openrouter"
//! overview_vendors = ["openrouter", "opencode-go", "deepseek"]
//!
//! [notifications]
//! enabled = true
//! threshold = 97
//!
//! [opencode-go]           # usage quota windows
//! enabled = true
//!
//! [openrouter]            # credits + recent-model activity
//! enabled = true
//! [[openrouter.accounts]] # optional named keys
//! label = "work"
//!
//! [deepseek]              # account balance
//! enabled = false
//! ```
//!
//! Every field is optional with sensible defaults — a missing config file is
//! treated as "use defaults". API keys are read from env vars (the relevant
//! `*_api_key_env` field lets the user override which env var name).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use serde::{Deserialize, Serialize};

use crate::balance::{DisplayPrefs, Headline};
use crate::error::{AppError, Result};
use crate::vendor::VendorId;

/// A misspelled section name is silently ignored without this: `[openrouer]`
/// leaves OpenRouter on its defaults and the user sees the wrong provider set
/// with no diagnostic. Denying unknown keys is deliberately applied at the
/// *section* level only — the set of sections is small and stable, whereas
/// denying unknown keys inside every section would hard-fail configs that
/// carry a field from a future or removed version.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub ui: UiConfig,
    pub notifications: NotificationsConfig,
    #[serde(rename = "opencode-go")]
    pub opencode_go: OpenCodeGoConfig,
    pub openrouter: OpenRouterConfig,
    pub deepseek: DeepseekConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct UiConfig {
    /// `None` → the first enabled provider.
    pub primary: Option<VendorId>,
    /// Which providers the report lists first, in this order. Enabled providers
    /// left off the list follow in the canonical order. `None` → every enabled
    /// provider, in the canonical order.
    pub overview_vendors: Option<Vec<VendorId>>,
}

/// Quota-threshold desktop notifications. On by default at 97%: a notification
/// is the signal for a window you are not looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct NotificationsConfig {
    pub enabled: bool,
    /// Percentage of a quota window at which a notification fires (1..=100;
    /// 100 means only an exhausted window notifies).
    pub threshold: u8,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 97,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct OpenCodeGoConfig {
    pub enabled: bool,
    pub api_key_env: String,
    pub api_key: Option<String>,
    /// Day of the month (1-28) the OpenCode Go billing cycle resets on, when
    /// the user has configured one. OpenCode Go reports only `resetsAt` for
    /// the monthly window, never its length (it follows the subscriber's
    /// billing date, 28/29/30/31 days). With this set the panel can compute an
    /// exact monthly window length and pace against it instead of guessing a
    /// 30-day cycle. `None` keeps the previous behaviour: a reset countdown
    /// with no window length.
    #[serde(default)]
    pub monthly_cycle_day: Option<u8>,
    /// Override the search list for the official OpenCode CLI's `auth.json`
    /// and OpenCode 2's `opencode.db`
    /// (`$XDG_DATA_HOME/opencode` or `~/.local/share/opencode`), for a
    /// non-standard install. The key is read read-only, never copied.
    pub auth_paths: Option<Vec<PathBuf>>,
    /// Unix timestamp (ms) of the monthly cycle start, set by
    /// `ai-usagebar opencode-go month-start`. When set, the monthly window
    /// paces from this date instead of a guessed subscriber-dependent bound.
    pub month_start_ms: Option<i64>,
    /// Fallback: number of calendar days back from the monthly reset to use
    /// as the month start when `month_start_ms` is unset.
    pub month_days: Option<u32>,
    /// Dollar limit for the rolling 5-hour window. Default 15.
    pub rolling_limit: Option<f64>,
    /// Dollar limit for the weekly window. Default 40.
    pub weekly_limit: Option<f64>,
    /// Dollar limit for the monthly window. Default 80.
    pub monthly_limit: Option<f64>,
}

impl Default for OpenCodeGoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key_env: "OPENCODE_GO_API_KEY".to_string(),
            api_key: None,
            monthly_cycle_day: None,
            auth_paths: None,
            month_start_ms: None,
            month_days: None,
            rolling_limit: Some(15.0),
            weekly_limit: Some(40.0),
            monthly_limit: Some(80.0),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct OpenRouterConfig {
    pub enabled: bool,
    /// Extra OpenRouter accounts beyond the default key. Each account gets a
    /// separate report entry and cache directory.
    pub accounts: Vec<ApiKeyAccount>,
    /// Whether the report includes the default (unnamed) key when named
    /// accounts exist. Ignored when `accounts` is empty so OpenRouter never
    /// loses its only entry.
    pub show_default_account: bool,
    pub api_key_env: String,
    pub api_key: Option<String>,
    /// Env var name for the optional OpenRouter *management* key. Only the
    /// `GET /api/v1/activity` call (recent models) uses it; when it resolves
    /// to nothing, that request is skipped instead of fired at a 401. The
    /// regular inference key is never sent to `/activity`.
    pub management_api_key_env: String,
    /// Which number goes on the bar. OpenRouter states its own denominator —
    /// credits purchased — so it is a quota provider and defaults to `percent`.
    /// See [`DisplayPrefs`].
    ///
    /// There is deliberately **no** `display_limit` here. A setting that is
    /// accepted and then always ignored is a footgun, and the one case where it
    /// would not be ignored — a free-tier account whose `total_credits` is 0 —
    /// is the case where honouring it would be wrong: the percentage on the bar
    /// comes from `OpenRouterSnapshot::consumed_pct`, which is 0 without
    /// credits, so a tank would name the headline `percent` and then show 0%
    /// for an account with money in it.
    pub headline: Headline,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            accounts: Vec::new(),
            show_default_account: true,
            api_key_env: "OPENROUTER_API_KEY".to_string(),
            api_key: None,
            management_api_key_env: "OPENROUTER_MANAGEMENT_API_KEY".to_string(),
            headline: Headline::Percent,
        }
    }
}

/// One named API-key account — the shape every `[[<provider>.accounts]]` array
/// shares. The default account continues to use the singular `api_key_env` /
/// `api_key` fields under the provider's own section.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ApiKeyAccount {
    /// Stable CLI/report label and account-scoped cache subdirectory.
    pub label: String,
    /// Optional environment variable containing this account's key.
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Inline fallback when the account environment variable is unset.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Optional environment variable containing this account's OpenRouter
    /// management key (unlocks the recent-models activity). Only OpenRouter
    /// consults it.
    #[serde(default)]
    pub management_api_key_env: Option<String>,
}

/// The `[[<slug>.accounts]]` lookup behind every API-key provider: find a named
/// account or fail loudly instead of falling back to the default key (which
/// would show the wrong account's usage).
fn api_key_account<'a>(
    slug: &str,
    accounts: &'a [ApiKeyAccount],
    label: &str,
) -> Result<&'a ApiKeyAccount> {
    validate_account_label_for(slug, label)?;
    accounts
        .iter()
        .find(|account| account.label == label)
        .ok_or_else(|| {
            let known: Vec<&str> = accounts
                .iter()
                .map(|account| account.label.as_str())
                .collect();
            AppError::Credentials(format!(
                "{slug} account {label:?} not found in [[{slug}.accounts]]; \
                 known labels: {known:?}"
            ))
        })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct DeepseekConfig {
    pub enabled: bool,
    /// Extra accounts beyond the default key. Each account gets a separate
    /// report entry and cache directory.
    pub accounts: Vec<ApiKeyAccount>,
    /// Whether the report includes the default (unnamed) key when named
    /// accounts exist. Ignored when `accounts` is empty so DeepSeek never
    /// loses its only entry.
    pub show_default_account: bool,
    pub api_key_env: String,
    pub api_key: Option<String>,
    /// Tank size in the currency `/user/balance` reports, so the remaining
    /// balance can be drawn as a meter. See [`DisplayPrefs`].
    pub display_limit: Option<f64>,
    /// Which number goes on the bar. See [`DisplayPrefs`].
    pub headline: Headline,
}

impl Default for DeepseekConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            accounts: Vec::new(),
            show_default_account: true,
            api_key_env: "DEEPSEEK_API_KEY".to_string(),
            api_key: None,
            display_limit: None,
            headline: Headline::Amount,
        }
    }
}

pub fn validate_account_label_for(vendor: &str, label: &str) -> Result<()> {
    const RESERVED: [&str; 4] = ["usage.json", ".stale", ".last_error", ".fetch.lock"];
    let bad = label.is_empty()
        || label == "."
        || label == ".."
        || label.contains(['/', '\\'])
        || label.contains(':')
        || label.chars().any(char::is_control)
        || RESERVED.contains(&label);
    if bad {
        return Err(AppError::Credentials(format!(
            "invalid {vendor} account label {label:?}: must be a non-empty name \
             without path separators, drive prefixes, control characters, or reserved cache names"
        )));
    }
    Ok(())
}

pub fn resolve_api_key(
    vendor_label: &str,
    env_var_name: &str,
    inline: Option<&str>,
) -> Result<String> {
    let section = match vendor_label {
        "OpenCode Go" => "[opencode-go]".to_string(),
        _ => format!("[{}]", vendor_label.to_lowercase()),
    };
    resolve_api_key_in_section(vendor_label, &section, env_var_name, inline)
}

/// The env-then-inline lookup without the "or fail" ending, for settings where
/// an absent key is a legitimate state.
pub fn optional_api_key(env_var_name: &str, inline: Option<&str>) -> Option<String> {
    optional_api_key_with(env_var_name, inline, |name| std::env::var(name).ok())
}

fn optional_api_key_with(
    env_var_name: &str,
    inline: Option<&str>,
    environment: impl FnOnce(&str) -> Option<String>,
) -> Option<String> {
    if is_valid_env_var_name(env_var_name)
        && let Some(v) = environment(env_var_name)
        && !v.is_empty()
    {
        return Some(v);
    }
    inline.filter(|v| !v.is_empty()).map(str::to_string)
}

fn resolve_api_key_in_section(
    vendor_label: &str,
    section: &str,
    env_var_name: &str,
    inline: Option<&str>,
) -> Result<String> {
    if let Some(key) = optional_api_key(env_var_name, inline) {
        return Ok(key);
    }
    let valid_env_name = is_valid_env_var_name(env_var_name);
    let advice = if valid_env_name {
        "set an API key in a valid environment variable or set `api_key`"
    } else {
        "fix the invalid `api_key_env` with a valid environment variable name or set `api_key`"
    };
    Err(AppError::Credentials(format!(
        "{vendor_label}: no API key. Either {advice} under {section} in {}.",
        config_path_hint()
    )))
}

pub(crate) fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl Config {
    /// Load from `~/.config/ai-usagebar/config.toml`. Returns defaults if the
    /// file doesn't exist; errors only on actual parse failures.
    pub fn load() -> Result<Self> {
        let Some(path) = resolved_path() else {
            return Ok(Self::default());
        };
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(s) => {
                let mut config: Self = toml::from_str(&s).map_err(|mut e| {
                    let span = e.span();
                    // Without its input the error's `Display` drops the quoted
                    // line but keeps the key path, `in deepseek.headline`.
                    e.set_input(None);
                    let summary = toml_error_summary(&s, span, &e.to_string());
                    AppError::Other(format!("config.toml: {summary}"))
                })?;
                // `~` is shell syntax, not path syntax: `PathBuf` keeps it
                // literally, so a documented `auth_paths = ["~/..."]` silently
                // pointed at a directory named `~`.
                config.expand_paths();
                config.validate()?;
                #[cfg(unix)]
                config.protect_inline_secrets(path)?;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(AppError::io_at(path, e)),
        }
    }

    fn expand_paths(&mut self) {
        if let Some(paths) = &mut self.opencode_go.auth_paths {
            for path in paths {
                *path = expand_tilde(path);
            }
        }
    }

    /// Explicitly enumerate every inline credential field. Adding a new
    /// credential provider must add it here so its config receives the same
    /// protection.
    #[cfg(unix)]
    fn has_inline_secrets(&self) -> bool {
        [
            self.openrouter.api_key.as_deref(),
            self.deepseek.api_key.as_deref(),
            self.opencode_go.api_key.as_deref(),
        ]
        .into_iter()
        .chain(
            Self::API_KEY_ACCOUNT_VENDORS
                .into_iter()
                .flat_map(|id| self.api_key_accounts(id).unwrap_or(&[]))
                .map(|account| account.api_key.as_deref()),
        )
        .any(|key| key.is_some_and(|key| !key.is_empty()))
    }

    #[cfg(unix)]
    fn protect_inline_secrets(&self, path: &Path) -> Result<()> {
        if !self.has_inline_secrets() {
            return Ok(());
        }

        let metadata = std::fs::metadata(path).map_err(|_| {
            AppError::Credentials(format!(
                "config at {} contains inline credentials but its permissions could not be checked; fix permissions or move credentials to environment variables",
                path.display()
            ))
        })?;
        if inline_key_permission_decision(metadata.mode()) == InlineKeyPermissionDecision::Tighten {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|_| {
                AppError::Credentials(format!(
                    "config at {} contains inline credentials but is group/other-readable and could not be tightened to 0600; fix permissions or move credentials to environment variables",
                    path.display()
                ))
            })?;
        }
        Ok(())
    }

    pub fn is_enabled(&self, id: VendorId) -> bool {
        match id {
            VendorId::OpenCodeGo => self.opencode_go.enabled,
            VendorId::Openrouter => self.openrouter.enabled,
            VendorId::Deepseek => self.deepseek.enabled,
        }
    }

    /// The environment variable this provider's API key is read from, honoring
    /// a per-provider `api_key_env` override. Matching on [`VendorId`] rather
    /// than on a section name is deliberate: a new provider that nobody adds
    /// here fails to compile, where a `_ =>` arm over `&str` sections would
    /// silently hand back the wrong default and report the provider as
    /// unconfigured for ever.
    pub fn api_key_env_for(&self, id: VendorId) -> &str {
        match id {
            VendorId::OpenCodeGo => &self.opencode_go.api_key_env,
            VendorId::Openrouter => &self.openrouter.api_key_env,
            VendorId::Deepseek => &self.deepseek.api_key_env,
        }
    }

    /// A non-empty inline `api_key` from this provider's config section. An
    /// empty string counts as unset, the same way the providers' own
    /// `resolve_api_key` treats it.
    pub fn inline_api_key(&self, id: VendorId) -> Option<&str> {
        let raw = match id {
            VendorId::OpenCodeGo => self.opencode_go.api_key.as_deref(),
            VendorId::Openrouter => self.openrouter.api_key.as_deref(),
            VendorId::Deepseek => self.deepseek.api_key.as_deref(),
        };
        raw.filter(|key| !key.is_empty())
    }

    /// The API-key providers that take a `[[<provider>.accounts]]` array.
    /// OpenCode Go is left out on purpose: its credential is one key (or the
    /// official CLI's own login), not an account list.
    pub const API_KEY_ACCOUNT_VENDORS: [VendorId; 2] = [VendorId::Openrouter, VendorId::Deepseek];

    /// The named `[[<provider>.accounts]]` array, or `None` for a provider that
    /// has no such array (see [`Self::API_KEY_ACCOUNT_VENDORS`]).
    pub fn api_key_accounts(&self, id: VendorId) -> Option<&[ApiKeyAccount]> {
        match id {
            VendorId::Openrouter => Some(&self.openrouter.accounts),
            VendorId::Deepseek => Some(&self.deepseek.accounts),
            VendorId::OpenCodeGo => None,
        }
    }

    /// Whether the default key keeps its entry next to the named accounts.
    /// `true` for every provider without an accounts array.
    pub fn show_default_api_key_account(&self, id: VendorId) -> bool {
        match id {
            VendorId::Openrouter => self.openrouter.show_default_account,
            VendorId::Deepseek => self.deepseek.show_default_account,
            VendorId::OpenCodeGo => true,
        }
    }

    /// The default key, or one named `[[<provider>.accounts]]` key, for a
    /// provider in [`Self::API_KEY_ACCOUNT_VENDORS`]. The default path is
    /// exactly the single-key resolution each provider always had, error text
    /// included. A label for a provider without the array fails loudly rather
    /// than falling back to the default key, which would show the wrong
    /// account's usage.
    pub fn resolve_account_api_key_for(&self, id: VendorId, label: Option<&str>) -> Result<String> {
        let name = id.display_name();
        let Some(label) = label else {
            return resolve_api_key(name, self.api_key_env_for(id), self.inline_api_key(id));
        };
        let slug = id.config_section();
        let account = api_key_account(slug, self.api_key_accounts(id).unwrap_or(&[]), label)?;
        // Names the account and its array section; never a configured value.
        resolve_api_key_in_section(
            &format!("{name} account {label:?}"),
            &format!("[[{slug}.accounts]]"),
            account.api_key_env.as_deref().unwrap_or(""),
            account.api_key.as_deref(),
        )
    }

    /// The optional OpenRouter *management* key for the default or a named
    /// account. `None` means none is configured, in which case the caller
    /// skips the `/api/v1/activity` request rather than firing a doomed 401 —
    /// the regular inference key is never sent there. A named account only
    /// resolves a management key when it names its own
    /// `management_api_key_env`; it never inherits the default account's.
    pub fn openrouter_management_key(&self, label: Option<&str>) -> Option<String> {
        let env_name = match label {
            Some(label) => api_key_account("openrouter", &self.openrouter.accounts, label)
                .ok()?
                .management_api_key_env
                .as_deref()?,
            None => self.openrouter.management_api_key_env.as_str(),
        };
        optional_api_key(env_name, None)
    }

    /// Bar-number settings for one provider.
    ///
    /// Only the balance providers declare these; OpenCode Go keeps the quota
    /// shape ([`DisplayPrefs::default`]) and is unaffected.
    pub fn display_prefs(&self, vendor: VendorId) -> DisplayPrefs {
        match vendor {
            VendorId::Deepseek => {
                DisplayPrefs::balance(self.deepseek.display_limit, self.deepseek.headline)
            }
            // No tank: OpenRouter reports its own credits. See
            // [`OpenRouterConfig::headline`].
            VendorId::Openrouter => DisplayPrefs::balance(None, self.openrouter.headline),
            VendorId::OpenCodeGo => DisplayPrefs::default(),
        }
    }

    pub fn enabled_vendors(&self) -> Vec<VendorId> {
        VendorId::all()
            .iter()
            .copied()
            .filter(|id| self.is_enabled(*id))
            .collect()
    }

    /// Validate cross-entry constraints that serde cannot express. Account
    /// labels are both selectors and report identities, so duplicates would
    /// make either destination ambiguous.
    pub fn validate(&self) -> Result<()> {
        if !(1..=100).contains(&self.notifications.threshold) {
            return Err(AppError::Other(format!(
                "[notifications] threshold must be between 1 and 100, got {}",
                self.notifications.threshold
            )));
        }
        // A tank size that cannot divide is a typo, and silently ignoring it
        // would draw a meter the user never asked for — or none, with no
        // diagnostic either way.
        if let Some(limit) = self.deepseek.display_limit
            && (!limit.is_finite() || limit <= 0.0)
        {
            return Err(AppError::Other(
                "[deepseek] display_limit must be finite and greater than zero; \
                 remove it to show the balance without a limit"
                    .into(),
            ));
        }
        // Every `[[<provider>.accounts]]` array follows OpenRouter's rules:
        // labels become cache subdirectories, stay unique as selectors and
        // report identities, and each entry names a key source.
        for id in Self::API_KEY_ACCOUNT_VENDORS {
            let slug = id.config_section();
            let mut labels = HashSet::new();
            for account in self.api_key_accounts(id).unwrap_or(&[]) {
                validate_account_label_for(slug, &account.label)?;
                if !labels.insert(&account.label) {
                    return Err(AppError::Credentials(format!(
                        "duplicate {slug} account label {:?}",
                        account.label
                    )));
                }
                let has_env = account
                    .api_key_env
                    .as_deref()
                    .is_some_and(|name| !name.is_empty());
                let has_inline = account
                    .api_key
                    .as_deref()
                    .is_some_and(|key| !key.is_empty());
                if !has_env && !has_inline {
                    return Err(AppError::Credentials(format!(
                        "{slug} account {:?} must set api_key_env or api_key",
                        account.label
                    )));
                }
            }
        }
        if let Some(day) = self.opencode_go.monthly_cycle_day
            && !(1..=28).contains(&day)
        {
            return Err(AppError::Other(format!(
                "[opencode-go] monthly_cycle_day must be between 1 and 28, got {day}"
            )));
        }
        Ok(())
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineKeyPermissionDecision {
    Ok,
    Tighten,
}

#[cfg(unix)]
fn inline_key_permission_decision(mode: u32) -> InlineKeyPermissionDecision {
    if mode & 0o077 == 0 {
        InlineKeyPermissionDecision::Ok
    } else {
        InlineKeyPermissionDecision::Tighten
    }
}

/// Set or update a boolean field in a TOML section, preserving comments and
/// formatting of unaffected nodes.
pub(crate) fn set_bool(
    doc: &mut toml_edit::DocumentMut,
    section: &str,
    key: &str,
    new_value: bool,
) -> Result<()> {
    let table = doc
        .entry(section)
        .or_insert_with(toml_edit::table)
        .as_table_like_mut()
        .ok_or_else(|| AppError::Other(format!("config.toml: [{section}] is not a table")))?;

    if let Some(item) = table.get_mut(key)
        && let Some(v) = item.as_value_mut()
    {
        // Keep a trailing `# comment` on the line being rewritten: the value
        // is the only thing that changed, and the note beside it is the
        // user's.
        let suffix = v.decor().suffix().cloned();
        *v = toml_edit::Value::from(new_value);
        v.decor_mut().set_prefix(" ");
        if let Some(suffix) = suffix {
            v.decor_mut().set_suffix(suffix);
        }
        return Ok(());
    }
    table.insert(key, toml_edit::value(new_value));
    Ok(())
}

/// Set, replace or remove a scalar field in a TOML section, preserving
/// comments and formatting of unaffected nodes. `None` removes the key so a
/// cleared preference does not linger as an empty string. The value keeps
/// its own TOML type on disk — an integer preference must not be quoted, or
/// `Config::load_from` rejects it.
pub(crate) fn set_value(
    doc: &mut toml_edit::DocumentMut,
    section: &str,
    key: &str,
    new_value: Option<toml_edit::Value>,
) -> Result<()> {
    let table = doc
        .entry(section)
        .or_insert_with(toml_edit::table)
        .as_table_like_mut()
        .ok_or_else(|| AppError::Other(format!("config.toml: [{section}] is not a table")))?;

    let Some(mut new_value) = new_value else {
        table.remove(key);
        return Ok(());
    };
    if let Some(item) = table.get_mut(key)
        && let Some(v) = item.as_value_mut()
    {
        let suffix = v.decor().suffix().cloned();
        new_value.decor_mut().set_prefix(" ");
        if let Some(suffix) = suffix {
            new_value.decor_mut().set_suffix(suffix);
        }
        *v = new_value;
        return Ok(());
    }
    table.insert(key, toml_edit::Item::Value(new_value));
    Ok(())
}

/// The validated core of the provider on/off writer, over an already-open
/// document: strictly `slug → VendorId → config_section()` — never a
/// caller-chosen section name — then the shared comment-preserving
/// [`set_bool`].
pub fn set_vendor_enabled_in_doc(
    doc: &mut toml_edit::DocumentMut,
    slug: &str,
    enabled: bool,
) -> Result<()> {
    let Some(vendor) = VendorId::from_slug(slug) else {
        return Err(AppError::Other(format!(
            "unknown provider {slug:?}: not a built-in provider"
        )));
    };
    set_bool(doc, vendor.config_section(), "enabled", enabled)
}

/// A TOML error as `line L, column C: message`. `toml` and `toml_edit` both
/// quote the offending source line in their `Display`, and in `config.toml`
/// that line can hold an inline `api_key`: one missing quote around a key put
/// the key in the report and on stderr. The position is enough to find the
/// mistake.
pub(crate) fn toml_error_summary(
    input: &str,
    span: Option<std::ops::Range<usize>>,
    message: &str,
) -> String {
    let message = message.trim_end();
    match span.and_then(|range| input.get(..range.start)) {
        Some(before) => {
            let line = before.matches('\n').count() + 1;
            let line_start = before.rfind('\n').map_or(0, |i| i + 1);
            let column = before[line_start..].chars().count() + 1;
            format!("line {line}, column {column}: {message}")
        }
        None => message.to_string(),
    }
}

/// Read `path` into a `toml_edit` document with comments intact. A missing
/// file is an empty document, so a writer can create the config from nothing;
/// any other I/O failure or a parse error is reported rather than clobbered.
pub(crate) fn read_config_document(path: &Path) -> Result<toml_edit::DocumentMut> {
    let original = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(AppError::io_at(path, error)),
    };
    if original.trim().is_empty() {
        return Ok(toml_edit::DocumentMut::new());
    }
    original.parse().map_err(|e: toml_edit::TomlError| {
        let summary = toml_error_summary(&original, e.span(), e.message());
        AppError::Other(format!("config.toml not parseable: {summary}"))
    })
}

/// Persist an edited config document: parent dir created, atomic
/// tempfile-and-rename write, and `chmod 600` on Unix because the file may
/// carry inline credentials. The one write path for every config editor.
pub(crate) fn write_config_document(path: &Path, doc: &toml_edit::DocumentMut) -> Result<()> {
    let bytes = doc.to_string();
    crate::cache::atomic_write(path, bytes.as_bytes())?;

    #[cfg(unix)]
    {
        if let Ok(meta) = std::fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(path, perms);
        }
    }
    Ok(())
}

pub fn default_path() -> Option<PathBuf> {
    let proj = directories::ProjectDirs::from("", "", "ai-usagebar")?;
    Some(proj.config_dir().join("config.toml"))
}

/// The Unix-conventional location, which is what every doc, the config
/// example, and the panel have always pointed at. On Linux it *is*
/// [`default_path`]; on macOS `ProjectDirs` resolves to
/// `~/Library/Application Support/…` instead, so the two diverge.
fn legacy_xdg_path() -> Option<PathBuf> {
    let home = crate::cache::home_dir().ok()?;
    Some(home.join(".config").join("ai-usagebar").join("config.toml"))
}

/// The config file actually in effect.
///
/// A `--config` override (see [`set_override_path`]) wins outright so a test
/// run never touches the real file. Otherwise [`default_path`] stays
/// canonical, but on macOS a file at the documented
/// `~/.config/ai-usagebar/config.toml` is honored when the canonical one does
/// not exist. The legacy file is never moved or rewritten: it may hold API
/// keys, and relocating a secret behind the user's back is not this tool's
/// business.
pub fn resolved_path() -> Option<PathBuf> {
    if let Some(path) = override_path() {
        return Some(path);
    }
    let canonical = default_path();
    if let Some(p) = &canonical
        && p.exists()
    {
        return canonical;
    }
    if let Some(legacy) = legacy_xdg_path()
        && legacy.exists()
    {
        return Some(legacy);
    }
    canonical
}

static PATH_OVERRIDE: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Point every config load, save, and hint at one explicit file — the
/// `--config` flag. Takes precedence over the canonical and legacy locations.
/// The file does not have to exist yet: loads treat it as defaults while
/// `settings apply` creates it. Process-wide, so call it once at startup before
/// any config is read.
pub fn set_override_path(path: &Path) {
    if let Ok(mut slot) = PATH_OVERRIDE.lock() {
        *slot = Some(path.to_path_buf());
    }
}

/// Drop the override again. Used only by tests so they can restore the
/// process-wide state they changed.
#[doc(hidden)]
pub fn clear_override_path() {
    if let Ok(mut slot) = PATH_OVERRIDE.lock() {
        *slot = None;
    }
}

fn override_path() -> Option<PathBuf> {
    PATH_OVERRIDE.lock().ok().and_then(|slot| slot.clone())
}

/// Value of a `--config=PATH` argument, split at the OS-string level so a
/// path with bytes Windows/Unix can store but UTF-8 cannot represent (an
/// undecodable filename on Unix, a lone surrogate on Windows) survives
/// intact instead of being mangled by `to_string_lossy`. `None` when the
/// argument is not in that form.
#[doc(hidden)]
pub fn config_flag_value(arg: &std::ffi::OsStr) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let rest = arg.as_bytes().strip_prefix(b"--config=")?;
        Some(std::ffi::OsString::from_vec(rest.to_vec()).into())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        const PREFIX: &[u16] = &[
            b'-' as u16,
            b'-' as u16,
            b'c' as u16,
            b'o' as u16,
            b'n' as u16,
            b'f' as u16,
            b'i' as u16,
            b'g' as u16,
            b'=' as u16,
        ];
        let wide: Vec<u16> = arg.encode_wide().collect();
        let rest = wide.strip_prefix(PREFIX)?;
        Some(std::ffi::OsString::from_wide(rest).into())
    }
    #[cfg(not(any(unix, windows)))]
    {
        Some(PathBuf::from(arg.to_str()?.strip_prefix("--config=")?))
    }
}

/// Expand a leading `~` (or `~/`) against the user's home directory. Anything
/// else — including `~user` — is left untouched.
fn expand_tilde(p: &Path) -> PathBuf {
    let Some(s) = p.to_str() else {
        return p.to_path_buf();
    };
    let rest = if s == "~" {
        ""
    } else if let Some(r) = s.strip_prefix("~/") {
        r
    } else {
        return p.to_path_buf();
    };
    match crate::cache::home_dir() {
        Ok(home) if rest.is_empty() => home,
        Ok(home) => home.join(rest),
        Err(_) => p.to_path_buf(),
    }
}

/// Resolved `config.toml` path as a string for user-facing messages. Uses the
/// platform's config dir (`directories::ProjectDirs`), so it reads correctly on
/// Linux, macOS, and Windows instead of hard-coding the Unix `~/.config` path.
/// Falls back to the bare filename if the path can't be resolved.
pub fn config_path_hint() -> String {
    resolved_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "config.toml".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(contents: &str) -> Config {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, contents).unwrap();
        Config::load_from(&path).unwrap()
    }

    fn load_err(contents: &str) -> AppError {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, contents).unwrap();
        Config::load_from(&path).unwrap_err()
    }

    #[test]
    fn a_missing_file_is_still_just_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope").join("config.toml");
        let c = Config::load_from(&missing).unwrap();
        assert!(c.is_enabled(VendorId::Openrouter));
        assert!(!c.is_enabled(VendorId::Deepseek));
        assert!(!c.is_enabled(VendorId::OpenCodeGo));
        assert_eq!(c.notifications.threshold, 97);
        assert!(c.notifications.enabled);
    }

    #[test]
    fn enabled_providers_come_back_in_canonical_order() {
        let c = load(
            r#"
            [deepseek]
            enabled = true
            [opencode-go]
            enabled = true
            [openrouter]
            enabled = false
            "#,
        );
        assert_eq!(
            c.enabled_vendors(),
            vec![VendorId::OpenCodeGo, VendorId::Deepseek]
        );
        assert!(!c.is_enabled(VendorId::Openrouter));
    }

    #[test]
    fn an_unknown_section_is_refused_rather_than_silently_ignored() {
        let err = load_err("[openruter]\nenabled = true\n");
        assert!(err.to_string().contains("unknown field"), "{err}");
    }

    #[test]
    fn the_panel_visible_settings_survive_a_round_trip() {
        let c = load(
            r#"
            [ui]
            primary = "deepseek"
            overview_vendors = ["deepseek", "openrouter"]
            [notifications]
            enabled = false
            threshold = 91
            [opencode-go]
            monthly_cycle_day = 12
            rolling_limit = 20.0
            "#,
        );
        assert_eq!(c.ui.primary, Some(VendorId::Deepseek));
        assert_eq!(
            c.ui.overview_vendors.as_deref(),
            Some(&[VendorId::Deepseek, VendorId::Openrouter][..])
        );
        assert!(!c.notifications.enabled);
        assert_eq!(c.notifications.threshold, 91);
        assert_eq!(c.opencode_go.monthly_cycle_day, Some(12));
        assert_eq!(c.opencode_go.rolling_limit, Some(20.0));
    }

    #[test]
    fn a_threshold_outside_one_to_hundred_is_refused() {
        let err = load_err("[notifications]\nthreshold = 0\n");
        assert!(err.to_string().contains("between 1 and 100"), "{err}");
        let err = load_err("[notifications]\nthreshold = 101\n");
        assert!(err.to_string().contains("between 1 and 100"), "{err}");
    }

    #[test]
    fn a_monthly_cycle_day_outside_one_to_twenty_eight_is_refused() {
        let err = load_err("[opencode-go]\nmonthly_cycle_day = 31\n");
        assert!(err.to_string().contains("between 1 and 28"), "{err}");
    }

    #[test]
    fn a_tank_that_cannot_divide_is_refused() {
        let err = load_err("[deepseek]\ndisplay_limit = 0.0\n");
        assert!(err.to_string().contains("greater than zero"), "{err}");
    }

    #[test]
    fn account_labels_are_validated_against_path_traversal() {
        let err = load_err(
            r#"
            [openrouter]
            [[openrouter.accounts]]
            label = "../etc"
            api_key = "k"
            "#,
        );
        assert!(
            err.to_string().contains("invalid openrouter account label"),
            "{err}"
        );
    }

    #[test]
    fn duplicate_account_labels_are_refused() {
        let err = load_err(
            r#"
            [openrouter]
            [[openrouter.accounts]]
            label = "work"
            api_key = "k1"
            [[openrouter.accounts]]
            label = "work"
            api_key = "k2"
            "#,
        );
        assert!(
            err.to_string()
                .contains("duplicate openrouter account label"),
            "{err}"
        );
    }

    #[test]
    fn an_account_without_a_key_source_is_refused() {
        let err = load_err(
            r#"
            [deepseek]
            [[deepseek.accounts]]
            label = "work"
            "#,
        );
        assert!(
            err.to_string().contains("must set api_key_env or api_key"),
            "{err}"
        );
    }

    #[test]
    fn a_named_account_resolves_its_own_key_and_a_missing_one_fails_loudly() {
        let c = load(
            r#"
            [openrouter]
            api_key = "default-key"
            [[openrouter.accounts]]
            label = "work"
            api_key = "work-key"
            "#,
        );
        assert_eq!(
            c.resolve_account_api_key_for(VendorId::Openrouter, None)
                .unwrap(),
            "default-key"
        );
        assert_eq!(
            c.resolve_account_api_key_for(VendorId::Openrouter, Some("work"))
                .unwrap(),
            "work-key"
        );
        let err = c
            .resolve_account_api_key_for(VendorId::Openrouter, Some("personal"))
            .unwrap_err();
        assert!(err.to_string().contains("not found"), "{err}");
        // A provider without the array names no account at all.
        assert!(c.api_key_accounts(VendorId::OpenCodeGo).is_none());
    }

    #[test]
    fn a_management_key_is_never_inherited_by_a_named_account() {
        let c = load(
            r#"
            [openrouter]
            management_api_key_env = "MANAGEMENT_KEY_PRESENT"
            [[openrouter.accounts]]
            label = "work"
            api_key = "work-key"
            "#,
        );
        // The default account reads the configured env var (absent here).
        assert_eq!(c.openrouter_management_key(None), None);
        assert_eq!(c.openrouter_management_key(Some("work")), None);
    }

    #[test]
    fn an_inline_key_tightens_the_file_to_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[deepseek]\napi_key = \"sk-test\"\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        Config::load_from(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn a_tilde_path_is_expanded_on_load() {
        let c = load("[opencode-go]\nauth_paths = [\"~/opencode/auth.json\"]\n");
        let paths = c.opencode_go.auth_paths.unwrap();
        assert!(!paths[0].to_string_lossy().starts_with('~'), "{paths:?}");
        assert!(paths[0].ends_with("opencode/auth.json"));
    }

    #[test]
    fn set_bool_keeps_the_trailing_comment() {
        let mut doc: toml_edit::DocumentMut =
            "[deepseek]\nenabled = false # my note\n".parse().unwrap();
        set_bool(&mut doc, "deepseek", "enabled", true).unwrap();
        let text = doc.to_string();
        assert!(text.contains("enabled = true"), "{text}");
        assert!(text.contains("# my note"), "{text}");
    }

    #[test]
    fn set_vendor_enabled_in_doc_refuses_a_name_that_is_not_a_provider() {
        let mut doc = toml_edit::DocumentMut::new();
        let err = set_vendor_enabled_in_doc(&mut doc, "openruter", true).unwrap_err();
        assert!(err.to_string().contains("unknown provider"), "{err}");
        set_vendor_enabled_in_doc(&mut doc, "openrouter", false).unwrap();
        assert!(doc.to_string().contains("[openrouter]"), "{doc}");
    }

    #[test]
    fn a_config_error_names_the_position_but_never_the_line() {
        let input = "a = 1\nkey = sk-secret\n";
        assert_eq!(
            toml_error_summary(input, Some(12..21), "string values must be quoted\n"),
            "line 2, column 7: string values must be quoted"
        );
        assert_eq!(toml_error_summary(input, None, "bad"), "bad");
        let err = load_err("[deepseek]\napi_key = sk-secret\n");
        assert!(!err.to_string().contains("sk-secret"), "{err}");
        assert!(err.to_string().contains("line 2"), "{err}");
    }

    #[test]
    fn the_config_override_wins_over_the_platform_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("explicit.toml");
        std::fs::write(&path, "[deepseek]\nenabled = true\n").unwrap();
        set_override_path(&path);
        assert_eq!(resolved_path().as_deref(), Some(path.as_path()));
        assert!(Config::load().unwrap().is_enabled(VendorId::Deepseek));
        clear_override_path();
        assert_ne!(resolved_path().as_deref(), Some(path.as_path()));
    }

    #[test]
    fn display_prefs_follow_the_provider_section() {
        let c = load(
            r#"
            [deepseek]
            display_limit = 200.0
            headline = "amount"
            [openrouter]
            display_limit = 5.0
            "#,
        );
        let deepseek = c.display_prefs(VendorId::Deepseek);
        assert_eq!(deepseek.display_limit, Some(200.0));
        assert_eq!(deepseek.headline, Headline::Amount);
        // OpenRouter refuses a tank: it states its own denominator.
        assert_eq!(c.display_prefs(VendorId::Openrouter).display_limit, None);
        assert_eq!(
            c.display_prefs(VendorId::OpenCodeGo),
            DisplayPrefs::default()
        );
    }
}
