//! Provider client — the entry list built from config, the per-entry fetch,
//! and the state each entry carries.
//!
//! One entry per enabled provider, plus one per named account of a provider
//! that supports account arrays. Everything that reads usage goes through
//! [`refresh_one`], so the cache/backoff/notification policy is one path.

use chrono::Utc;
use reqwest::Client;

use crate::cache::DEFAULT_TTL;
use crate::config::Config;
use crate::error::Result;
use crate::vendor::{VendorId, VendorOutcome};

/// What we have per entry: a snapshot plus fetch metadata, or the error that
/// kept us from getting one.
///
/// `Ready` is boxed because the snapshot is much larger than the other two
/// variants (silences `clippy::large_enum_variant`).
#[derive(Debug, Clone)]
pub enum TabState {
    Loading,
    Ready(Box<ReadyTab>),
    Error {
        message: String,
        /// Provider plan already known from credentials when the quota fetch
        /// failed. Absent when the provider has no plan without a snapshot.
        plan: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct ReadyTab {
    pub snapshot: crate::usage::VendorSnapshot,
    pub stale: bool,
    pub last_error: Option<(u16, String)>,
    /// Absolute moment the cache was written (i.e. the API response landed).
    /// Snapshotted once at `TabState` build time so the rendered "Updated …"
    /// timestamp stays stable across redraws instead of drifting with the
    /// passing wall clock.
    pub fetched_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Bar-number settings this provider was configured with — the tank size a
    /// prepaid balance is metered against, and which of the two numbers goes on
    /// the bar. Resolved from config at fetch time rather than stored in the
    /// snapshot, so editing config.toml takes effect on the next refresh
    /// instead of waiting for the cache to expire.
    pub display: crate::balance::DisplayPrefs,
    /// OpenCode Go billing-cycle reset day (1-28), when configured. Only set
    /// for OpenCode Go entries.
    pub monthly_cycle_day: Option<u8>,
}

impl TabState {
    pub fn error(message: impl AsRef<str>) -> Self {
        Self::error_with_plan(message, None)
    }

    pub fn error_with_plan(message: impl AsRef<str>, plan: Option<String>) -> Self {
        let plan = plan.and_then(|plan| {
            let cleaned = crate::display::sanitize_untrusted_field(plan.trim());
            if cleaned.is_empty() || cleaned.eq_ignore_ascii_case("unknown") {
                None
            } else {
                Some(cleaned)
            }
        });
        Self::Error {
            message: message.as_ref().to_string(),
            plan,
        }
    }
}

/// Identity of one entry: a provider, and the named account when the provider
/// supports account arrays. `account: None` is the provider's default key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TabId {
    pub vendor: VendorId,
    pub account: Option<String>,
}

impl TabId {
    /// A plain provider entry (its default key).
    pub fn vendor(vendor: VendorId) -> Self {
        Self {
            vendor,
            account: None,
        }
    }

    /// A named account of a provider that supports account arrays.
    pub fn account_for(vendor: VendorId, label: impl Into<String>) -> Self {
        Self {
            vendor,
            account: Some(label.into()),
        }
    }

    pub fn vendor_id(&self) -> VendorId {
        self.vendor
    }
}

/// Expand enabled providers into the entry list. OpenRouter and DeepSeek yield
/// their default key followed by configured named accounts; OpenCode Go is a
/// single entry. With no extra accounts and `overview_vendors` unset, the
/// result equals `config.enabled_vendors()` in canonical order. When the list
/// is set, listed providers lead and unlisted enabled providers follow in that
/// same canonical order.
pub fn tabs_from_config(config: &Config) -> Vec<TabId> {
    let mut tabs = Vec::new();
    for vendor in vendors_for_tabs(config) {
        if let Some(accounts) = config.api_key_accounts(vendor) {
            if config.show_default_api_key_account(vendor) || accounts.is_empty() {
                tabs.push(TabId::vendor(vendor));
            }
            for account in accounts {
                tabs.push(TabId::account_for(vendor, account.label.clone()));
            }
        } else {
            tabs.push(TabId::vendor(vendor));
        }
    }
    tabs
}

/// Enabled providers in `[ui] overview_vendors` order. Unlisted providers keep
/// the canonical order and follow the listed ones. Unset means canonical.
fn vendors_for_tabs(config: &Config) -> Vec<VendorId> {
    let enabled = config.enabled_vendors();
    let Some(wanted) = config.ui.overview_vendors.as_ref() else {
        return enabled;
    };
    let mut ordered = Vec::with_capacity(enabled.len());
    for vendor in wanted {
        if enabled.contains(vendor) && !ordered.contains(vendor) {
            ordered.push(*vendor);
        }
    }
    for vendor in enabled {
        if !ordered.contains(&vendor) {
            ordered.push(vendor);
        }
    }
    ordered
}

/// Fetch one entry and turn the outcome into its state.
pub async fn refresh_one(client: &Client, config: &Config, tab: &TabId) -> TabState {
    match build_outcome(client, config, tab).await {
        Ok(outcome) => {
            // Resolve the cache age (a duration from "now" at fetch time) into
            // an absolute instant ONCE. Without this, the projection would
            // recompute `Utc::now() - cache_age` on every draw and the
            // displayed time would tick upward in real time instead of holding
            // at the last refresh.
            let now = Utc::now();
            let off_the_wire = outcome.off_the_wire();
            let fetched_at = outcome
                .cache_age
                .map(|age| now - chrono::Duration::from_std(age).unwrap_or_default());
            let state = TabState::Ready(Box::new(ReadyTab {
                snapshot: outcome.snapshot,
                stale: outcome.stale,
                last_error: outcome.last_error.map(|(code, message)| {
                    (code, crate::display::sanitize_untrusted_field(&message))
                }),
                fetched_at,
                display: config.display_prefs(tab.vendor),
                monthly_cycle_day: match tab.vendor {
                    VendorId::OpenCodeGo => config.opencode_go.monthly_cycle_day,
                    _ => None,
                },
            }));
            // Quota-threshold notifications: wire-fresh outcomes only — never
            // cached, stale, or failed ones — and best-effort by construction:
            // `run` returns nothing and cannot change this function's result.
            if off_the_wire
                && config.notifications.enabled
                && let Some(input) = crate::notify::RefreshInput::from_tab(tab, &state, now)
            {
                let _ = crate::notify::run(input, config.notifications.threshold).await;
            }
            state
        }
        Err(e) => TabState::error_with_plan(
            crate::display::sanitize_untrusted_field(&e.user_message()),
            e.plan().map(str::to_string),
        ),
    }
}

/// Key and cache for a provider entry: a named account's key with its own
/// `<slug>/<label>` cache, or the default key with the provider-root cache,
/// whose path never moves.
fn api_key_and_cache(
    config: &Config,
    vendor: VendorId,
    account: Option<&str>,
) -> Result<(String, crate::cache::Cache)> {
    let api_key = config.resolve_account_api_key_for(vendor, account)?;
    let cache = match account {
        Some(label) => crate::cache::Cache::for_vendor_account(vendor.slug(), label)?,
        None => crate::cache::Cache::for_vendor(vendor.slug())?,
    };
    Ok((api_key, cache))
}

async fn build_outcome(client: &Client, config: &Config, tab: &TabId) -> Result<VendorOutcome> {
    let vendor = tab.vendor;
    match vendor {
        VendorId::Openrouter => {
            let (api_key, cache) = api_key_and_cache(config, vendor, tab.account.as_deref())?;
            let management_key = config.openrouter_management_key(tab.account.as_deref());
            let endpoints = crate::openrouter::fetch::Endpoints::default();
            let outcome = crate::openrouter::fetch_snapshot(
                client,
                &api_key,
                management_key.as_deref(),
                &cache,
                &endpoints,
                DEFAULT_TTL,
            )
            .await?;
            Ok(outcome.into())
        }
        VendorId::Deepseek => {
            let (api_key, cache) = api_key_and_cache(config, vendor, tab.account.as_deref())?;
            let endpoints = crate::deepseek::fetch::Endpoints::default();
            let outcome =
                crate::deepseek::fetch_snapshot(client, &api_key, &cache, &endpoints, DEFAULT_TTL)
                    .await?;
            Ok(outcome.into())
        }
        VendorId::OpenCodeGo => {
            let api_key = crate::opencode_go::creds::resolve_key(
                &config.opencode_go.api_key_env,
                config.opencode_go.api_key.as_deref(),
                config.opencode_go.auth_paths.as_deref(),
            )?;
            let cache = crate::cache::Cache::for_vendor("opencode-go")?;
            let endpoints = crate::opencode_go::fetch::Endpoints::default();
            let (outcome, _history) = crate::opencode_go::fetch::fetch_snapshot(
                client,
                &api_key,
                &cache,
                &endpoints,
                DEFAULT_TTL,
            )
            .await?;
            Ok(outcome.into())
        }
    }
}

impl From<crate::openrouter::fetch::FetchOutcome> for VendorOutcome {
    fn from(outcome: crate::openrouter::fetch::FetchOutcome) -> Self {
        outcome.map(crate::usage::VendorSnapshot::Openrouter)
    }
}

impl From<crate::deepseek::fetch::FetchOutcome> for VendorOutcome {
    fn from(outcome: crate::deepseek::fetch::FetchOutcome) -> Self {
        outcome.map(crate::usage::VendorSnapshot::Deepseek)
    }
}

impl From<crate::opencode_go::fetch::FetchOutcome> for VendorOutcome {
    fn from(outcome: crate::opencode_go::fetch::FetchOutcome) -> Self {
        outcome.map(crate::usage::VendorSnapshot::OpenCodeGo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiKeyAccount;

    fn config_with(account_vendor: Option<(&str, bool)>, accounts: &[&str]) -> Config {
        let mut text = String::from("[opencode-go]\nenabled = false\n");
        if let Some((section, show_default)) = account_vendor {
            text.push_str(&format!(
                "[{section}]\nenabled = true\nshow_default_account = {show_default}\n"
            ));
            for label in accounts {
                text.push_str(&format!(
                    "[[{section}.accounts]]\nlabel = \"{label}\"\napi_key = \"k-{label}\"\n"
                ));
            }
        }
        let config: Config = toml::from_str(&text).unwrap();
        config.validate().unwrap();
        config
    }

    #[test]
    fn every_enabled_provider_gets_exactly_one_default_entry() {
        let config: Config = toml::from_str(
            "[opencode-go]\nenabled = true\n[openrouter]\nenabled = true\n[deepseek]\nenabled = true\n",
        )
        .unwrap();
        assert_eq!(
            tabs_from_config(&config),
            vec![
                TabId::vendor(VendorId::OpenCodeGo),
                TabId::vendor(VendorId::Openrouter),
                TabId::vendor(VendorId::Deepseek),
            ]
        );
    }

    #[test]
    fn named_accounts_follow_the_default_entry() {
        let config = config_with(Some(("openrouter", true)), &["work", "personal"]);
        assert_eq!(
            tabs_from_config(&config),
            vec![
                TabId::vendor(VendorId::Openrouter),
                TabId::account_for(VendorId::Openrouter, "work"),
                TabId::account_for(VendorId::Openrouter, "personal"),
            ]
        );
    }

    #[test]
    fn the_default_entry_is_suppressible_but_never_to_an_empty_provider() {
        // OpenRouter stays on in this config, so both entries appear.
        let config = config_with(Some(("deepseek", false)), &["work"]);
        assert_eq!(
            tabs_from_config(&config),
            vec![
                TabId::vendor(VendorId::Openrouter),
                TabId::account_for(VendorId::Deepseek, "work"),
            ]
        );
        let mut empty = config_with(Some(("deepseek", false)), &[]);
        empty.deepseek.accounts.clear();
        assert_eq!(
            tabs_from_config(&empty),
            vec![
                TabId::vendor(VendorId::Openrouter),
                TabId::vendor(VendorId::Deepseek),
            ]
        );
    }

    #[test]
    fn overview_vendors_order_listed_providers_first() {
        let mut config: Config = toml::from_str(
            "[opencode-go]\nenabled = true\n[openrouter]\nenabled = true\n[deepseek]\nenabled = true\n",
        )
        .unwrap();
        config.ui.overview_vendors = Some(vec![VendorId::Deepseek]);
        assert_eq!(
            tabs_from_config(&config),
            vec![
                TabId::vendor(VendorId::Deepseek),
                TabId::vendor(VendorId::OpenCodeGo),
                TabId::vendor(VendorId::Openrouter),
            ]
        );
    }

    #[test]
    fn a_disabled_provider_contributes_no_entries() {
        let config = config_with(Some(("openrouter", true)), &["work"]);
        assert_eq!(
            tabs_from_config(&config),
            vec![
                TabId::vendor(VendorId::Openrouter),
                TabId::account_for(VendorId::Openrouter, "work"),
            ]
        );
        // Account arrays only exist on the providers that declare them.
        assert!(config.api_key_accounts(VendorId::OpenCodeGo).is_none());
        assert!(config.api_key_accounts(VendorId::Deepseek).is_some());
        let _: &[ApiKeyAccount] = &config.openrouter.accounts;
    }
}
