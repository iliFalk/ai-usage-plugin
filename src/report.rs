//! `ai-usagebar usage` — quota and balance for every configured provider, in
//! one pass.
//!
//! This walks the provider client's entry list, refreshes each one, projects
//! its snapshot into labelled rows, and prints either the machine-readable
//! document the Omarchy panel parses or a plain text report. Deliberately thin:
//! [`crate::provider::tabs_from_config`] decides what is configured,
//! [`crate::provider::refresh_one`] fetches and caches it, and
//! [`crate::sections::project`] projects it. No provider ever needs to know
//! this file exists.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;

use crate::config::Config;
use crate::provider::{TabId, TabState, refresh_one, tabs_from_config};
use crate::sections::{Section, project};

/// Matches the projection's default; only affects the pacing note appended to
/// a metric's detail line.
const PACE_TOLERANCE: u32 = 5;

/// Version of the tolerant, machine-readable `usage --json` contract.
/// Increment only when an incompatible change cannot be represented by adding
/// or omitting fields.
const USAGE_SCHEMA_VERSION: u8 = 1;

/// One configured provider or named account.
struct Entry {
    id: String,
    name: String,
    display_name: String,
    /// The provider's `{vendor_short}` code. Frontends take it from here rather
    /// than keeping their own table.
    short_name: String,
    /// The provider's bar glyph. Same rule as `short_name`: one table, in Rust.
    icon: String,
    plan: Option<String>,
    sections: Vec<ReportSection>,
    error: Option<String>,
    stale: bool,
    fetched_at: Option<DateTime<Utc>>,
    /// Per-window usage samples, present only for a provider whose windows
    /// accumulate (OpenCode Go). Absent — not null — everywhere else: a balance
    /// has no window to plot, and inventing one would put a fake curve on the
    /// panel.
    history: Option<crate::opencode_go::history::ReportHistory>,
}

/// Lossless machine-readable projection of one panel row. `metrics` remains
/// available in JSON as a convenience view over only the gauge rows; callers
/// that need every reported value consume `sections`.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReportSection {
    Metric {
        label: String,
        percent: u16,
        value: String,
        detail: String,
        /// Which of `percent` and `value` this metric puts on the bar —
        /// `"percent"` or `"value"`. A consumer that honours it draws the named
        /// one and leaves the other in the detail line, rather than inferring a
        /// balance row from its label.
        headline: String,
        severity: String,
        reset_at: Option<DateTime<Utc>>,
        /// Full length of the reset window in seconds, present only when the
        /// provider states it exactly (a rolling 5h/7d window). A frontend that
        /// wants a pace indicator needs both this and `reset_at`; a calendar
        /// month or an unstated window omits the field rather than guessing.
        #[serde(skip_serializing_if = "Option::is_none")]
        window_secs: Option<u64>,
    },
    Text {
        label: String,
        value: String,
    },
    Block {
        label: String,
        body: Vec<String>,
    },
    Spacer,
}

impl ReportSection {
    fn label(&self) -> Option<&str> {
        match self {
            Self::Metric { label, .. } | Self::Text { label, .. } | Self::Block { label, .. } => {
                Some(label)
            }
            Self::Spacer => None,
        }
    }
}

pub async fn run(json: bool) -> i32 {
    let (entries, primary, overview) = match collect_entries().await {
        Ok(collected) => collected,
        Err(message) => {
            eprintln!("ai-usagebar usage: {message}");
            return 1;
        }
    };

    if json {
        println!(
            "{}",
            render_json_document(&entries, primary, overview.as_deref())
        );
    } else {
        print!("{}", render_text(&entries));
    }
    report_exit_code(&entries)
}

async fn collect_entries()
-> std::result::Result<(Vec<Entry>, Option<&'static str>, Option<Vec<&'static str>>), String> {
    let config = Config::load().map_err(|error| error.user_message())?;
    let client = http_client().map_err(|error| error.user_message())?;
    let tabs = tabs_from_config(&config);
    if tabs.is_empty() {
        return Err(format!(
            "no providers enabled in {}",
            crate::config::config_path_hint()
        ));
    }
    let entries = collect_entries_for(&client, &config, &tabs).await;
    let overview = config
        .ui
        .overview_vendors
        .as_ref()
        .map(|vendors| vendors.iter().map(|vendor| vendor.slug()).collect());
    Ok((
        entries,
        config.ui.primary.map(|vendor| vendor.slug()),
        overview,
    ))
}

/// The shared HTTP client: one timeout and one redirect policy, so a provider
/// cannot answer a redirect question differently from its siblings.
fn http_client() -> crate::error::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(crate::vendor::HTTP_CLIENT_TIMEOUT)
        .redirect(crate::vendor::same_origin_redirect_policy())
        .build()
        .map_err(|error| crate::error::AppError::Other(format!("http client: {error}")))
}

/// Sequential on purpose: several entries share a per-provider cache lock, and
/// firing every named account at one endpoint at once is a good way to get
/// rate-limited for no gain on a handful of entries.
async fn collect_entries_for(
    client: &reqwest::Client,
    config: &Config,
    tabs: &[TabId],
) -> Vec<Entry> {
    let mut entries = Vec::with_capacity(tabs.len());
    for tab in tabs {
        let state = refresh_one(client, config, tab).await;
        entries.push(entry_from_state(tab, &state, Utc::now()));
    }
    entries
}

fn entry_from_state(tab: &TabId, state: &TabState, now: DateTime<Utc>) -> Entry {
    let mut entry = Entry {
        id: tab_id(tab),
        name: tab_name(tab),
        display_name: tab_display_name(tab),
        short_name: tab.vendor.short_name().to_string(),
        icon: tab.vendor.bar_icon().to_string(),
        plan: match state {
            TabState::Error { plan, .. } => plan
                .as_deref()
                .map(crate::display::sanitize_untrusted_field)
                .filter(|plan| !plan.is_empty()),
            _ => None,
        },
        sections: Vec::new(),
        error: match state {
            TabState::Error { message, .. } => {
                Some(crate::display::sanitize_untrusted_field(message))
            }
            _ => None,
        },
        stale: matches!(state, TabState::Ready(ready) if ready.stale),
        fetched_at: match state {
            TabState::Ready(ready) => ready.fetched_at,
            _ => None,
        },
        history: match state {
            TabState::Ready(ready) => ready.history.report(ready.monthly_cycle_day),
            _ => None,
        },
    };
    // The error is already a first-class entry field. Do not duplicate an
    // interactive retry instruction as report data.
    if entry.error.is_some() {
        return entry;
    }
    for projected in project(state, now, PACE_TOLERANCE) {
        match projected.section {
            Section::Title { left, .. } => entry.plan = Some(left),
            Section::Metric {
                label,
                pct,
                value_label,
                footnote,
                severity,
                ..
            } => {
                entry.sections.push(ReportSection::Metric {
                    label,
                    percent: pct,
                    value: value_label,
                    detail: footnote,
                    headline: projected.headline.as_str().into(),
                    severity: severity.as_str().into(),
                    reset_at: projected.reset_at,
                    window_secs: projected
                        .window
                        .filter(|window| window.num_seconds() > 0)
                        .map(|window| window.num_seconds() as u64),
                });
            }
            Section::Text { label, value } => {
                entry.sections.push(ReportSection::Text { label, value });
            }
            Section::Block { label, body } => {
                entry.sections.push(ReportSection::Block { label, body });
            }
            Section::Spacer => entry.sections.push(ReportSection::Spacer),
        }
    }
    entry
}

/// Process status after a complete document has been printed.
///
/// Per-entry fetch/auth failures are data inside the document, not a command
/// failure. Empty is not a document — [`collect_entries`] already fails before
/// this when nothing is enabled.
fn report_exit_code(entries: &[Entry]) -> i32 {
    i32::from(entries.is_empty())
}

/// Stable machine id, also the entry half of the notification dedupe key:
/// `<provider>@<label>` for named accounts, the bare slug for a default key.
pub(crate) fn tab_id(tab: &TabId) -> String {
    match &tab.account {
        Some(account) => format!("{}@{account}", tab.vendor.slug()),
        None => tab.vendor.slug().to_string(),
    }
}

fn tab_name(tab: &TabId) -> String {
    format_tab_name(tab, tab.vendor.slug())
}

fn tab_display_name(tab: &TabId) -> String {
    format_tab_name(tab, tab.vendor.display_name())
}

fn format_tab_name(tab: &TabId, provider_name: &str) -> String {
    let name = match &tab.account {
        Some(account) => format!("{provider_name} · {account}"),
        None => provider_name.to_string(),
    };
    crate::display::sanitize_untrusted_field(&name)
}

/// Resolve the `primary` the report should name to an id `entries` actually
/// carries.
///
/// `config.ui.primary` is a provider slug, but with named accounts the entry
/// ids are `provider@account`, so the raw slug names an id no entry has —
/// `primary: "openrouter"` next to `openrouter@work`. The first entry of that
/// provider (the bare slug, or the first `{slug}@…` account) wins; a slug with
/// no matching entry is kept as-is, because the config naming a disabled or
/// absent provider is information worth reporting, not something to paper over
/// with a guess. `None` (unset) stays `None`.
fn resolve_primary(primary: Option<&str>, entries: &[Entry]) -> Option<String> {
    primary.map(|slug| {
        entries
            .iter()
            .find(|entry| entry.id == slug || entry.id.starts_with(&format!("{slug}@")))
            .map(|entry| entry.id.clone())
            .unwrap_or_else(|| slug.to_string())
    })
}

fn render_json_document(
    entries: &[Entry],
    primary: Option<&str>,
    overview: Option<&[&str]>,
) -> String {
    let mut value = json!({
        "schema_version": USAGE_SCHEMA_VERSION,
        "primary": resolve_primary(primary, entries),
        "entries": json_rows(entries),
    });
    if let Some(order) = overview {
        value["overview_vendors"] = json!(order);
    }
    value.to_string()
}

fn json_rows(entries: &[Entry]) -> Vec<serde_json::Value> {
    entries
        .iter()
        .map(|entry| {
            let metrics = entry
                .sections
                .iter()
                .filter_map(|section| match section {
                    ReportSection::Metric {
                        label,
                        percent,
                        value,
                        detail,
                        headline,
                        severity,
                        reset_at,
                        window_secs,
                    } => {
                        let mut metric = json!({
                            "label": label,
                            "percent": percent,
                            "value": value,
                            "detail": detail,
                            "headline": headline,
                            "severity": severity,
                            "reset_at": reset_at,
                        });
                        // Same rule as the `sections` serializer: absent, not null.
                        if let Some(secs) = window_secs {
                            metric["window_secs"] = json!(secs);
                        }
                        Some(metric)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            let mut row = json!({
                "id": entry.id,
                "name": entry.name,
                "display_name": entry.display_name,
                "short_name": entry.short_name,
                "icon": entry.icon,
                "plan": entry.plan,
                "status": if entry.error.is_some() { "error" } else { "ready" },
                "error": entry.error,
                "stale": entry.stale,
                "fetched_at": entry.fetched_at,
                "metrics": metrics,
                "sections": entry.sections,
            });
            // Absent, never null — the same rule the optional section fields
            // follow, so a panel from before this field keeps working.
            if let Some(history) = &entry.history {
                row["history"] = json!(history);
            }
            row
        })
        .collect()
}

fn render_text(entries: &[Entry]) -> String {
    // Widest label across every entry, so the value column lines up down the
    // whole report rather than per-section.
    let width = entries
        .iter()
        .flat_map(|entry| entry.sections.iter())
        .filter_map(ReportSection::label)
        .map(crate::display::text_width)
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    for entry in entries {
        out.push_str(&entry.name);
        if let Some(plan) = &entry.plan {
            out.push_str(&format!("   {plan}"));
        }
        out.push('\n');
        if let Some(error) = &entry.error {
            out.push_str(&format!("  ! {error}\n\n"));
            continue;
        }
        if !entry
            .sections
            .iter()
            .any(|section| !matches!(section, ReportSection::Spacer))
        {
            out.push_str("  (nothing reported)\n\n");
            continue;
        }
        let mut body = String::new();
        let mut pending_spacer = false;
        for section in &entry.sections {
            if matches!(section, ReportSection::Spacer) {
                pending_spacer |= !body.is_empty();
                continue;
            }
            if pending_spacer {
                body.push('\n');
                pending_spacer = false;
            }
            match section {
                ReportSection::Metric {
                    label,
                    value,
                    detail,
                    ..
                } => {
                    let label = crate::display::pad_end(label, width);
                    let value = format!("{value:>9}");
                    if detail.is_empty() {
                        body.push_str(&format!("  {label}  {value}\n"));
                    } else {
                        body.push_str(&format!("  {label}  {value}   {detail}\n"));
                    }
                }
                ReportSection::Text { label, value } => {
                    if label.is_empty() {
                        body.push_str(&format!("  {}\n", value.trim_start()));
                    } else if value.is_empty() {
                        body.push_str(&format!("  {label}\n"));
                    } else {
                        let label = crate::display::pad_end(label, width);
                        body.push_str(&format!("  {label}  {value}\n"));
                    }
                }
                ReportSection::Block { label, body: lines } => {
                    body.push_str(&format!("  {label}\n"));
                    for line in lines {
                        body.push_str(&format!("    {line}\n"));
                    }
                }
                ReportSection::Spacer => unreachable!(),
            }
        }
        out.push_str(&body);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balance::DisplayPrefs;
    use crate::provider::ReadyTab;
    use crate::usage::{DeepseekSnapshot, OpenRouterSnapshot, VendorSnapshot};
    use crate::vendor::VendorId;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap()
    }

    fn ready(snapshot: VendorSnapshot) -> TabState {
        TabState::Ready(Box::new(ReadyTab {
            snapshot,
            stale: false,
            last_error: None,
            fetched_at: Some(now() - chrono::Duration::seconds(20)),
            display: DisplayPrefs::default(),
            monthly_cycle_day: None,
            history: Default::default(),
        }))
    }

    fn openrouter() -> VendorSnapshot {
        VendorSnapshot::Openrouter(OpenRouterSnapshot {
            label: "OR".into(),
            total_credits: 100.0,
            total_usage: 25.0,
            usage_daily: 1.0,
            usage_weekly: 5.0,
            usage_monthly: 25.0,
            is_free_tier: false,
            limit: None,
            limit_remaining: None,
            recent_models: Vec::new(),
        })
    }

    fn deepseek() -> VendorSnapshot {
        VendorSnapshot::Deepseek(DeepseekSnapshot {
            is_available: true,
            balance: 12.5,
            granted: 2.5,
            topped_up: 10.0,
            currency: "USD".into(),
        })
    }

    fn document(entries: &[Entry], primary: Option<&str>) -> serde_json::Value {
        serde_json::from_str(&render_json_document(entries, primary, Some(&["deepseek"]))).unwrap()
    }

    #[test]
    fn entry_ids_and_names_carry_the_provider_and_its_account() {
        let tab = TabId::vendor(VendorId::Openrouter);
        assert_eq!(tab_id(&tab), "openrouter");
        assert_eq!(tab_name(&tab), "openrouter");
        assert_eq!(tab_display_name(&tab), "OpenRouter");
        let account = TabId::account_for(VendorId::Openrouter, "work");
        assert_eq!(tab_id(&account), "openrouter@work");
        assert_eq!(tab_name(&account), "openrouter · work");
        assert_eq!(tab_display_name(&account), "OpenRouter · work");
    }

    #[test]
    fn a_ready_entry_carries_identity_status_and_sections() {
        let tab = TabId::vendor(VendorId::Openrouter);
        let entry = entry_from_state(&tab, &ready(openrouter()), now());
        assert_eq!(entry.id, "openrouter");
        assert_eq!(entry.short_name, "opr");
        assert!(!entry.icon.is_empty());
        assert!(!entry.stale);
        assert!(entry.error.is_none());
        assert!(entry.fetched_at.is_some());
        entry
            .sections
            .iter()
            .find(|section| {
                matches!(section, ReportSection::Metric { label, percent, severity, .. }
                    if label == "Credit balance" && *percent == 25 && severity == "low")
            })
            .expect("a credit balance metric");
    }

    #[test]
    fn a_failed_entry_reports_the_error_and_no_rows() {
        let tab = TabId::vendor(VendorId::Deepseek);
        let state = TabState::error_with_plan("no API key", Some("DeepSeek".to_string()));
        let entry = entry_from_state(&tab, &state, now());
        assert_eq!(entry.error.as_deref(), Some("no API key"));
        assert_eq!(entry.plan.as_deref(), Some("DeepSeek"));
        assert!(entry.sections.is_empty());
        let value = document(std::slice::from_ref(&entry), None);
        assert_eq!(value["entries"][0]["status"], "error");
        assert_eq!(value["entries"][0]["error"], "no API key");
    }

    fn opencode_go_with_history() -> TabState {
        let mut state = ready(VendorSnapshot::OpenCodeGo(
            crate::opencode_go::types::parse_usage(&serde_json::json!({
                "usage": {
                    "weekly": {
                        "status": "ok",
                        "percent": 42.0,
                        "resetsAt": "2026-08-20T00:00:00Z"
                    },
                    "monthly": {
                        "status": "ok",
                        "percent": 61.0,
                        "resetsAt": "2026-09-15T00:00:00Z"
                    }
                }
            }))
            .expect("fixture parses"),
        ));
        if let TabState::Ready(ready) = &mut state {
            ready.history.weekly = crate::opencode_go::history::WindowHistory {
                resets_at: Some("2026-08-20T00:00:00+00:00".into()),
                points: vec![
                    crate::opencode_go::history::HistoryPoint {
                        t: 1_787_000_000_000,
                        p: 30.0,
                    },
                    crate::opencode_go::history::HistoryPoint {
                        t: 1_787_003_600_000,
                        p: 42.0,
                    },
                ],
            };
        }
        state
    }

    #[test]
    fn only_the_opencode_go_entry_carries_usage_history() {
        let opencode = entry_from_state(
            &TabId::vendor(VendorId::OpenCodeGo),
            &opencode_go_with_history(),
            now(),
        );
        let weekly = &opencode.history.as_ref().expect("history").weekly;
        let weekly = weekly.as_ref().expect("weekly window");
        assert_eq!(weekly.points.len(), 2);
        assert_eq!(weekly.points[1].p, 42.0);
        // 7 days before the reset instant.
        assert_eq!(weekly.start, 1_786_579_200_000);

        let openrouter = entry_from_state(
            &TabId::vendor(VendorId::Openrouter),
            &ready(openrouter()),
            now(),
        );
        assert!(
            openrouter.history.is_none(),
            "a balance has no window to plot"
        );

        let entries = vec![opencode, openrouter];
        let value = document(&entries, None);
        let rows = value["entries"].as_array().expect("entries");
        assert_eq!(
            rows[0]["history"]["weekly"]["resetsAt"],
            "2026-08-20T00:00:00+00:00"
        );
        assert_eq!(
            rows[0]["history"]["weekly"]["points"][0]["t"],
            1_787_000_000_000.0
        );
        assert_eq!(rows[0]["history"]["weekly"]["points"][0]["p"], 30.0);
        assert!(
            rows[0]["history"].get("rolling").is_none(),
            "a window with no samples is absent"
        );
        assert!(
            rows[1].get("history").is_none(),
            "no history key at all, not a null one"
        );
    }

    #[test]
    fn a_cached_entry_is_marked_stale() {
        let mut state = ready(deepseek());
        if let TabState::Ready(ready) = &mut state {
            ready.stale = true;
            ready.last_error = Some((429, "rate limited".into()));
        }
        let entry = entry_from_state(&TabId::vendor(VendorId::Deepseek), &state, now());
        assert!(entry.stale);
        assert!(
            entry.sections.iter().any(|section| matches!(
                section,
                ReportSection::Text { label, .. } if label == "HTTP 429"
            )),
            "{:?}",
            entry.sections.len()
        );
    }

    #[test]
    fn the_document_names_the_schema_the_primary_and_the_overview_order() {
        let entries = vec![
            entry_from_state(
                &TabId::vendor(VendorId::Deepseek),
                &ready(deepseek()),
                now(),
            ),
            entry_from_state(
                &TabId::account_for(VendorId::Openrouter, "work"),
                &ready(openrouter()),
                now(),
            ),
        ];
        let value = document(&entries, Some("openrouter"));
        assert_eq!(value["schema_version"], 1);
        // A slug with named accounts resolves to the first of its entries.
        assert_eq!(value["primary"], "openrouter@work");
        assert_eq!(value["overview_vendors"], json!(["deepseek"]));
        assert_eq!(value["entries"][0]["short_name"], "dsk");
        assert_eq!(value["entries"][1]["id"], "openrouter@work");
    }

    #[test]
    fn an_unknown_primary_slug_is_reported_as_configured() {
        assert_eq!(
            resolve_primary(Some("opencode-go"), &[]),
            Some("opencode-go".to_string())
        );
        assert_eq!(resolve_primary(None, &[]), None);
    }

    #[test]
    fn the_text_report_prints_one_block_per_entry() {
        let entries = vec![
            entry_from_state(
                &TabId::vendor(VendorId::Deepseek),
                &ready(deepseek()),
                now(),
            ),
            entry_from_state(
                &TabId::vendor(VendorId::Openrouter),
                &ready(openrouter()),
                now(),
            ),
        ];
        let text = render_text(&entries);
        assert!(text.starts_with("deepseek   DeepSeek"), "{text}");
        assert!(text.contains("$12.50"), "{text}");
        assert!(text.contains("openrouter   OR"), "{text}");
        assert!(text.contains("Credit balance"), "{text}");
    }

    #[test]
    fn an_empty_document_is_a_failed_run() {
        assert_eq!(report_exit_code(&[]), 1);
        let entries = vec![entry_from_state(
            &TabId::vendor(VendorId::Deepseek),
            &ready(deepseek()),
            now(),
        )];
        assert_eq!(report_exit_code(&entries), 0);
    }
}
