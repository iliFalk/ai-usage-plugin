//! Snapshot → sections projection.
//!
//! Each provider turns its snapshot into an ordered sequence of [`Section`]s:
//! a metric row (percentage, severity, reset) or a free-form text/block row.
//! This is the only place per-provider metric order exists — the report
//! serializes these rows and the panel draws them, so adding a row is one
//! edit here rather than one per frontend.
//!
//! Reset metadata travels beside the public row (never inside it): a
//! machine-readable frontend needs the absolute reset instant and the window
//! length to pace a metric, and a text-only consumer does not.

use chrono::{DateTime, Datelike, Utc};

use crate::balance::{self, DisplayPrefs, MetricHeadline};
use crate::countdown;
use crate::format::{clamp_pct, local_time_hms, money, usd};
use crate::pacing::{self, PaceSeverity, severity_for};
use crate::provider::TabState;
use crate::usage::VendorSnapshot;

/// One row of a provider's panel body.
pub enum Section {
    /// Title row at the top. `left` is the plan/provider label; `right` is an
    /// optional right-aligned annotation, used for the "Updated HH:MM:SS"
    /// timestamp so it shares the title row.
    Title { left: String, right: Option<String> },
    /// A metric: label + gauge + value annotation + dim footnote.
    Metric {
        label: String,
        pct: u16,
        severity: PaceSeverity,
        value_label: String,
        footnote: String,
    },
    /// Free-form key/value text line.
    Text { label: String, value: String },
    /// A label followed by a multi-line dim block (no gauge).
    Block { label: String, body: Vec<String> },
    /// Visual spacer (one blank row).
    Spacer,
}

/// The public row plus the metadata a machine-readable frontend needs.
pub(crate) struct SectionProjection {
    pub section: Section,
    pub reset_at: Option<DateTime<Utc>>,
    /// Full length of the metric's reset window, when it is known exactly
    /// (a rolling 5h/7d window). `None` for calendar periods and for quotas
    /// whose window length the provider never states — a frontend can pace a
    /// metric only when this is `Some`.
    pub window: Option<chrono::Duration>,
    /// Which of the metric's two numbers the frontend puts on the bar. Every
    /// metric is a percentage unless its provider says otherwise, so this is
    /// [`MetricHeadline::Percent`] by default — the bar draws it and leaves
    /// the money figure in the detail. A prepaid-balance provider flips it,
    /// and that declaration is what the frontend reads instead of guessing
    /// from the label.
    pub headline: MetricHeadline,
}

struct SectionBuilder(Vec<SectionProjection>);

impl SectionBuilder {
    fn projection(section: Section) -> SectionProjection {
        SectionProjection {
            section,
            reset_at: None,
            window: None,
            headline: MetricHeadline::Percent,
        }
    }

    fn new(sections: Vec<Section>) -> Self {
        Self(
            sections
                .into_iter()
                .map(|section| {
                    assert!(
                        !matches!(section, Section::Metric { .. }),
                        "metric sections must declare reset metadata with push_metric"
                    );
                    Self::projection(section)
                })
                .collect(),
        )
    }

    fn push(&mut self, section: Section) {
        assert!(
            !matches!(section, Section::Metric { .. }),
            "metric sections must declare reset metadata with push_metric"
        );
        self.0.push(Self::projection(section));
    }

    /// A metric whose window length is not known exactly (a calendar month,
    /// a provider-defined billing period, or no stated window at all).
    fn push_metric(&mut self, section: Section, reset_at: Option<DateTime<Utc>>) {
        assert!(matches!(section, Section::Metric { .. }));
        let mut row = Self::projection(section);
        row.reset_at = reset_at;
        self.0.push(row);
    }

    /// A metric whose provider chose which of its two numbers goes on the bar.
    fn push_metric_with_headline(
        &mut self,
        section: Section,
        reset_at: Option<DateTime<Utc>>,
        headline: MetricHeadline,
    ) {
        assert!(matches!(section, Section::Metric { .. }));
        let mut row = Self::projection(section);
        row.reset_at = reset_at;
        row.headline = headline;
        self.0.push(row);
    }

    /// A metric on a window of exactly `window` length, so a frontend can
    /// compute how far through the window `reset_at` sits.
    fn push_metric_in_window(
        &mut self,
        section: Section,
        reset_at: Option<DateTime<Utc>>,
        window: chrono::Duration,
    ) {
        assert!(matches!(section, Section::Metric { .. }));
        let mut row = Self::projection(section);
        row.reset_at = reset_at;
        row.window = Some(window);
        self.0.push(row);
    }
}

/// Project one entry's state into its ordered rows, with the absolute
/// fetched-at instant stamped into the title row.
pub(crate) fn project(
    tab: &TabState,
    now: DateTime<Utc>,
    pace_tolerance: u32,
) -> Vec<SectionProjection> {
    let mut sections = match tab {
        TabState::Loading => SectionBuilder::new(vec![
            Section::Spacer,
            Section::Text {
                label: "".into(),
                value: "  Loading…".into(),
            },
        ]),
        TabState::Error { message: e, plan } => {
            let mut rows = Vec::new();
            if let Some(plan) = plan {
                rows.push(Section::Title {
                    left: plan.clone(),
                    right: None,
                });
            }
            rows.extend([
                Section::Spacer,
                Section::Text {
                    label: "Error".into(),
                    value: e.clone(),
                },
            ]);
            SectionBuilder::new(rows)
        }
        TabState::Ready(r) => {
            let snapshot = &r.snapshot;
            let last_error = &r.last_error;
            let prefs = r.display;
            let mut sections = match snapshot {
                VendorSnapshot::Openrouter(s) => openrouter_sections(s, prefs),
                VendorSnapshot::Deepseek(s) => deepseek_sections(s, prefs),
                VendorSnapshot::OpenCodeGo(s) => {
                    opencode_go_sections(s, now, pace_tolerance, r.monthly_cycle_day)
                }
            };
            // Inject the (already-absolute) fetched-at instant into the title
            // row, right-aligned. Pre-snapshotted in provider::refresh_one so
            // it doesn't drift between redraws.
            let updated = match r.fetched_at {
                Some(at) => format!("Updated {}", local_time_hms(at)),
                None => "Updated —".to_string(),
            };
            if let Some(SectionProjection {
                section: Section::Title { right, .. },
                ..
            }) = sections.0.first_mut()
            {
                *right = Some(updated);
            }
            // Error footer (when present) still lives in the body.
            if let Some((label, msg)) = warning_label(last_error) {
                sections.push(Section::Spacer);
                sections.push(Section::Text { label, value: msg });
            }
            sections
        }
    };
    for projected in &mut sections.0 {
        sanitize_section(&mut projected.section);
    }
    sections.0
}

fn sanitize_section(section: &mut Section) {
    let clean = |value: &mut String| {
        *value = crate::display::sanitize_untrusted_field(value);
    };
    match section {
        Section::Title { left, right } => {
            clean(left);
            if let Some(right) = right {
                clean(right);
            }
        }
        Section::Metric {
            label,
            value_label,
            footnote,
            ..
        } => {
            clean(label);
            clean(value_label);
            clean(footnote);
        }
        Section::Text { label, value } => {
            clean(label);
            clean(value);
        }
        Section::Block { label, body } => {
            clean(label);
            for line in body {
                clean(line);
            }
        }
        Section::Spacer => {}
    }
}

/// Translate cache diagnostics at the presentation boundary. Cache files keep
/// their established `(u16, String)` form: a non-zero code is HTTP, a zero one
/// is a plain warning.
fn warning_label(last_error: &Option<(u16, String)>) -> Option<(String, String)> {
    let (code, message) = last_error.as_ref()?;
    if *code != 0 {
        return Some((format!("HTTP {code}"), message.clone()));
    }
    if message.is_empty() {
        return None;
    }
    Some(("Warning".into(), message.clone()))
}

fn push_balance_headline(
    v: &mut SectionBuilder,
    label: &str,
    remaining: f64,
    currency: &str,
    money_severity: PaceSeverity,
    api_limit: Option<f64>,
    prefs: DisplayPrefs,
) {
    let amount = money(remaining, currency);
    let Some(limit) = balance::denominator(api_limit, prefs.display_limit) else {
        // No denominator from either source: nothing to meter against, so the
        // row stays free text rather than becoming a fabricated 0% gauge.
        v.push(Section::Text {
            label: label.into(),
            value: amount,
        });
        return;
    };

    let pct = balance::consumed_pct(limit, remaining);
    let headline = balance::resolve_headline(prefs.headline, Some(limit));
    let of_limit = money(limit, currency);
    let (value_label, footnote) = match headline {
        MetricHeadline::Percent => (
            format!("{pct}%"),
            format!("{amount} of {of_limit} left ({pct}% used)"),
        ),
        MetricHeadline::Value => (
            amount.clone(),
            format!("{pct}% of {of_limit} used ({amount} left)"),
        ),
    };
    v.push_metric_with_headline(
        Section::Metric {
            label: label.into(),
            pct,
            severity: money_severity.max(severity_for(i32::from(pct))),
            value_label,
            footnote,
        },
        None,
        headline,
    );
}

fn openrouter_sections(
    s: &crate::usage::OpenRouterSnapshot,
    prefs: DisplayPrefs,
) -> SectionBuilder {
    let mut v = SectionBuilder::new(vec![Section::Title {
        left: s.label.clone(),
        right: None,
    }]);
    let pct = s.consumed_pct().clamp(0, 100) as u16;
    // OpenRouter states its own denominator — credits purchased — so there is
    // no user tank to fall back to and `None` is passed literally rather than
    // `prefs.display_limit`. A free-tier-only account buys nothing, so
    // `total_credits` is 0, `denominator` is `None`, and the money stays on the
    // bar. Routing `prefs.display_limit` in here instead would let a tank take
    // over in exactly that case and name the headline `percent` — while `pct`
    // below still came from `consumed_pct()`, which is 0 without credits. The
    // bar would read "0%" for an account with money in it.
    let denominator = balance::denominator(Some(s.total_credits), None);
    v.push(Section::Spacer);
    v.push_metric_with_headline(
        Section::Metric {
            label: "Credit balance".into(),
            pct,
            // One severity policy for every frontend: this value is what the
            // panel colours its row with, so an account in debt must not read
            // as a healthy one.
            severity: crate::openrouter::types::severity(s),
            value_label: usd(s.balance()),
            footnote: format!(
                "{} of {} used ({pct}%)",
                usd(s.total_usage),
                usd(s.total_credits)
            ),
        },
        None,
        balance::resolve_headline(prefs.headline, denominator),
    );
    v.push(Section::Spacer);
    v.push(Section::Block {
        label: "Usage by period".into(),
        body: vec![format!(
            "today ${:.2} · week ${:.2} · month ${:.2}",
            s.usage_daily, s.usage_weekly, s.usage_monthly
        )],
    });
    if let (Some(limit), Some(rem)) = (s.limit, s.limit_remaining) {
        v.push(Section::Spacer);
        v.push(Section::Block {
            label: "Per-key limit".into(),
            body: vec![format!("{} of {} remaining", usd(rem), usd(limit))],
        });
    }
    v.push(Section::Spacer);
    v.push(Section::Block {
        label: "Tier".into(),
        body: vec![if s.is_free_tier {
            "free tier".into()
        } else {
            "paid tier".into()
        }],
    });
    if !s.recent_models.is_empty() {
        v.push(Section::Spacer);
        v.push(Section::Block {
            label: "Recent models".into(),
            body: s.recent_models.clone(),
        });
    }
    v
}

fn deepseek_sections(s: &crate::usage::DeepseekSnapshot, prefs: DisplayPrefs) -> SectionBuilder {
    let currency = &s.currency;
    let fmt = |v: f64| money(v, currency);
    let avail = if s.is_available {
        "available"
    } else {
        "unavailable"
    };
    let mut v = SectionBuilder::new(vec![Section::Title {
        left: "DeepSeek".into(),
        right: None,
    }]);
    v.push(Section::Spacer);
    // `/user/balance` reports money remaining and no denominator at all, so
    // `api_limit` is unconditionally `None` here.
    push_balance_headline(
        &mut v,
        "Balance",
        s.balance,
        currency,
        crate::deepseek::types::severity(s),
        None,
        prefs,
    );
    v.push(Section::Block {
        label: "Breakdown".into(),
        body: vec![format!(
            "granted {} · topped-up {}",
            fmt(s.granted),
            fmt(s.topped_up)
        )],
    });
    v.push(Section::Spacer);
    v.push(Section::Block {
        label: "API".into(),
        body: vec![avail.into()],
    });
    v
}

fn opencode_go_sections(
    s: &crate::opencode_go::types::Usage,
    now: DateTime<Utc>,
    tol: u32,
    monthly_cycle_day: Option<u8>,
) -> SectionBuilder {
    use crate::opencode_go::types::{ROLLING_WINDOW, WEEKLY_WINDOW};

    let mut sections = SectionBuilder::new(vec![Section::Title {
        left: "OpenCode Go".into(),
        right: None,
    }]);
    let mut any = false;
    for (label, window, duration) in [
        ("Rolling (5h)", s.rolling.as_ref(), ROLLING_WINDOW),
        ("Weekly (7d)", s.weekly.as_ref(), WEEKLY_WINDOW),
    ] {
        let Some(window) = window else {
            continue;
        };
        any = true;
        let pct = i32::from(clamp_pct(window.percent));
        let projected = crate::usage::UsageWindow {
            utilization_pct: pct,
            resets_at: Some(window.resets_at),
            window_duration: duration,
        };
        push_window(&mut sections, label, &projected, now, tol, true);
    }
    // Monthly keeps its reset countdown. When the user configured a
    // `monthly_cycle_day` the panel can compute an exact window length from
    // the previous cycle reset to this one, so machine-readable frontends get
    // `window_secs` and can pace against it instead of guessing a 30-day cycle.
    if let Some(window) = s.monthly.as_ref() {
        any = true;
        let pct = clamp_pct(window.percent);
        let metric = Section::Metric {
            label: "Monthly".into(),
            pct,
            severity: severity_for(i32::from(pct)),
            value_label: format!("{pct}%"),
            footnote: format!(
                "Resets in {}",
                countdown::format(Some(window.resets_at), now)
            ),
        };
        match monthly_cycle_day.and_then(|day| monthly_window_secs(window.resets_at, day)) {
            Some(duration) => {
                sections.push_metric_in_window(metric, Some(window.resets_at), duration);
            }
            None => {
                sections.push_metric(metric, Some(window.resets_at));
            }
        }
    }
    if !any {
        sections.push(Section::Spacer);
        sections.push(Section::Text {
            label: "".into(),
            value: "  no usage windows reported".into(),
        });
    }
    sections
}

/// Compute the exact length of one OpenCode Go monthly billing cycle from its
/// next reset instant and the configured cycle day (1-28). OpenCode Go reports
/// `resetsAt` but never the cycle length, so the panel derives the window from
/// the subscriber's billing day: the window starts at the previous cycle day
/// (clamped to the previous month's last day, so Feb 30 never happens) and
/// ends at `resets_at`. Returns None when the dates don't form a positive
/// window, in which case callers keep the previous behaviour (no `window_secs`).
fn monthly_window_secs(resets_at: DateTime<Utc>, cycle_day: u8) -> Option<chrono::Duration> {
    let end = resets_at.naive_utc().date();
    let (prev_year, prev_month) = if end.month() == 1 {
        (end.year() - 1, 12u32)
    } else {
        (end.year(), end.month() - 1)
    };
    // Last day of the previous month, used to clamp cycle_day (e.g. day 30 in
    // February, day 31 in a 30-day month). The month after the previous one is
    // `end`'s own month, so "first day of this month minus one day" is the
    // previous month's last day and handles the December/January rollover.
    let prev_month_last = chrono::NaiveDate::from_ymd_opt(end.year(), end.month(), 1)?
        .checked_sub_days(chrono::Days::new(1))?;
    let days_in_prev_month = prev_month_last.day();
    let start_day = cycle_day.min(days_in_prev_month as u8);
    let start = chrono::NaiveDate::from_ymd_opt(prev_year, prev_month, start_day.into())?;
    let seconds = end.signed_duration_since(start).num_seconds();
    if seconds > 0 {
        Some(chrono::Duration::seconds(seconds))
    } else {
        None
    }
}

#[cfg(test)]
mod monthly_window_secs_tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn cycle_length_is_days_in_the_start_month() {
        // Cycle day 15, reset on Sep 15: window is Aug 15 -> Sep 15 = 31 days.
        let resets = Utc.with_ymd_and_hms(2026, 9, 15, 0, 0, 0).unwrap();
        assert_eq!(
            monthly_window_secs(resets, 15),
            Some(chrono::Duration::days(31))
        );
        // Cycle day 15, reset on Oct 15: window is Sep 15 -> Oct 15 = 30 days.
        let resets = Utc.with_ymd_and_hms(2026, 10, 15, 0, 0, 0).unwrap();
        assert_eq!(
            monthly_window_secs(resets, 15),
            Some(chrono::Duration::days(30))
        );
        // Cycle day 15, reset on Mar 15: window is Feb 15 -> Mar 15 = 28 days (2026 not a leap year).
        let resets = Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap();
        assert_eq!(
            monthly_window_secs(resets, 15),
            Some(chrono::Duration::days(28))
        );
        // Cycle day 29 in February clamps to Feb's last day: Feb 28 -> Mar 29 = 29 days.
        let resets = Utc.with_ymd_and_hms(2026, 3, 29, 0, 0, 0).unwrap();
        assert_eq!(
            monthly_window_secs(resets, 29),
            Some(chrono::Duration::days(29))
        );
        // Cycle day 1 reset on Jan 1: window is Dec 1 -> Jan 1 = 31 days.
        let resets = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(
            monthly_window_secs(resets, 1),
            Some(chrono::Duration::days(31))
        );
    }

    #[test]
    fn unknown_cycle_day_returns_none() {
        let resets = Utc.with_ymd_and_hms(2026, 9, 15, 0, 0, 0).unwrap();
        assert_eq!(monthly_window_secs(resets, 0), None);
    }
}

fn push_window(
    sections: &mut SectionBuilder,
    label: &str,
    w: &crate::usage::UsageWindow,
    now: DateTime<Utc>,
    tol: u32,
    show_pacing: bool,
) {
    let pct = w.utilization_pct.clamp(0, 100) as u16;
    let reset_text = countdown::format(w.resets_at, now);
    let footnote = if show_pacing {
        let p = pacing::calc(w.utilization_pct, w.resets_at, now, w.window_duration, tol);
        format!(
            "Resets in {} · {}% elapsed · {}",
            reset_text, p.elapsed_pct, p.point_label
        )
    } else {
        format!("Resets in {}", reset_text)
    };
    sections.push(Section::Spacer);
    sections.push_metric_in_window(
        Section::Metric {
            label: label.into(),
            pct,
            severity: severity_for(pct as i32),
            value_label: format!("{pct}%"),
            footnote,
        },
        w.resets_at,
        w.window_duration,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::{DeepseekSnapshot, OpenRouterSnapshot};
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap()
    }

    fn ready(snapshot: VendorSnapshot) -> TabState {
        ready_with(snapshot, DisplayPrefs::default())
    }

    fn ready_with(snapshot: VendorSnapshot, display: DisplayPrefs) -> TabState {
        TabState::Ready(Box::new(crate::provider::ReadyTab {
            snapshot,
            stale: false,
            last_error: None,
            fetched_at: Some(now() - chrono::Duration::seconds(15)),
            display,
            monthly_cycle_day: None,
        }))
    }

    fn openrouter(total_credits: f64, total_usage: f64) -> VendorSnapshot {
        VendorSnapshot::Openrouter(OpenRouterSnapshot {
            label: "OR".into(),
            total_credits,
            total_usage,
            usage_daily: 1.0,
            usage_weekly: 5.0,
            usage_monthly: total_usage,
            is_free_tier: false,
            limit: None,
            limit_remaining: None,
            recent_models: Vec::new(),
        })
    }

    fn labels(sections: &[SectionProjection]) -> Vec<String> {
        sections
            .iter()
            .filter_map(|projection| match &projection.section {
                Section::Metric { label, .. }
                | Section::Text { label, .. }
                | Section::Block { label, .. } => Some(label.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn openrouter_always_has_balance_metric_and_period_block() {
        let sections = project(&ready(openrouter(100.0, 25.0)), now(), 5);
        assert!(matches!(sections[0].section, Section::Title { .. }));
        let labels = labels(&sections);
        assert!(labels.contains(&"Credit balance".to_string()), "{labels:?}");
        assert!(
            labels.contains(&"Usage by period".to_string()),
            "{labels:?}"
        );
    }

    #[test]
    fn openrouter_debt_reaches_the_panel_row_red_and_signed() {
        let sections = project(&ready(openrouter(0.0, 5.71)), now(), 5);
        let metric = sections
            .iter()
            .find_map(|projection| match &projection.section {
                Section::Metric {
                    label,
                    value_label,
                    severity,
                    footnote,
                    ..
                } if label == "Credit balance" => Some((value_label, severity, footnote)),
                _ => None,
            })
            .expect("no credit balance metric");
        assert_eq!(metric.0, "-$5.71");
        assert_eq!(*metric.1, PaceSeverity::Critical);
        assert_eq!(metric.2, "$5.71 of $0.00 used (0%)");
    }

    #[test]
    fn deepseek_without_a_tank_keeps_its_plain_text_row() {
        let snapshot = VendorSnapshot::Deepseek(DeepseekSnapshot {
            is_available: true,
            balance: 5.5,
            granted: 5.5,
            topped_up: 0.0,
            currency: "USD".into(),
        });
        let sections = project(&ready(snapshot), now(), 5);
        assert!(sections.iter().any(|projection| matches!(
            &projection.section,
            Section::Text { label, value } if label == "Balance" && value == "$5.50"
        )));
    }

    #[test]
    fn a_tank_meters_the_deepseek_balance_without_moving_the_money_off_the_bar() {
        let snapshot = VendorSnapshot::Deepseek(DeepseekSnapshot {
            is_available: true,
            balance: 50.0,
            granted: 50.0,
            topped_up: 0.0,
            currency: "USD".into(),
        });
        let prefs = DisplayPrefs::balance(Some(200.0), crate::balance::Headline::Amount);
        let sections = project(&ready_with(snapshot, prefs), now(), 5);
        let row = sections
            .iter()
            .find(|projection| {
                matches!(&projection.section, Section::Metric { label, .. } if label == "Balance")
            })
            .expect("a balance metric");
        assert_eq!(row.headline, MetricHeadline::Value);
        match &row.section {
            Section::Metric {
                pct,
                value_label,
                footnote,
                ..
            } => {
                assert_eq!(*pct, 75);
                assert_eq!(value_label, "$50.00");
                assert_eq!(footnote, "75% of $200.00 used ($50.00 left)");
            }
            _ => panic!("expected a metric"),
        }
    }

    #[test]
    fn opencode_go_projects_its_three_windows_with_exact_lengths() {
        use crate::opencode_go::types::{Usage, Window};
        let window = |percent: f64, hours: i64| Window {
            status: "ok".into(),
            percent,
            resets_at: now() + chrono::Duration::hours(hours),
        };
        let snapshot = VendorSnapshot::OpenCodeGo(Usage {
            rolling: Some(window(12.0, 3)),
            weekly: Some(window(45.0, 48)),
            monthly: Some(window(78.0, 240)),
        });
        let sections = project(&ready(snapshot), now(), 5);
        let metrics: Vec<_> = sections
            .iter()
            .filter_map(|projection| match &projection.section {
                Section::Metric { label, pct, .. } => {
                    Some((label.clone(), *pct, projection.window))
                }
                _ => None,
            })
            .collect();
        assert_eq!(metrics.len(), 3, "{:?}", labels(&sections));
        assert_eq!(metrics[0].0, "Rolling (5h)");
        assert_eq!(metrics[0].2, Some(chrono::Duration::hours(5)));
        assert_eq!(metrics[1].0, "Weekly (7d)");
        assert_eq!(metrics[1].2, Some(chrono::Duration::days(7)));
        // The monthly window has no length until a cycle day is configured.
        assert_eq!(metrics[2].0, "Monthly");
        assert_eq!(metrics[2].2, None);
    }

    #[test]
    fn cycle_length_is_days_in_the_start_month() {
        let resets = Utc.with_ymd_and_hms(2026, 9, 15, 0, 0, 0).unwrap();
        assert_eq!(
            monthly_window_secs(resets, 15),
            Some(chrono::Duration::days(31))
        );
        let resets = Utc.with_ymd_and_hms(2026, 10, 15, 0, 0, 0).unwrap();
        assert_eq!(
            monthly_window_secs(resets, 15),
            Some(chrono::Duration::days(30))
        );
        let resets = Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap();
        assert_eq!(
            monthly_window_secs(resets, 15),
            Some(chrono::Duration::days(28))
        );
        let resets = Utc.with_ymd_and_hms(2026, 3, 29, 0, 0, 0).unwrap();
        assert_eq!(
            monthly_window_secs(resets, 29),
            Some(chrono::Duration::days(29))
        );
        let resets = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(
            monthly_window_secs(resets, 1),
            Some(chrono::Duration::days(31))
        );
        // Day 0 is not a cycle day.
        let resets = Utc.with_ymd_and_hms(2026, 9, 15, 0, 0, 0).unwrap();
        assert_eq!(monthly_window_secs(resets, 0), None);
    }

    #[test]
    fn a_monthly_cycle_day_gives_the_monthly_metric_its_window() {
        let usage = crate::opencode_go::types::Usage {
            rolling: None,
            weekly: None,
            monthly: Some(crate::opencode_go::types::Window {
                status: "ok".into(),
                percent: 50.0,
                resets_at: Utc.with_ymd_and_hms(2026, 6, 10, 0, 0, 0).unwrap(),
            }),
        };
        let mut tab = ready(VendorSnapshot::OpenCodeGo(usage));
        if let TabState::Ready(ready) = &mut tab {
            ready.monthly_cycle_day = Some(10);
        }
        let sections = project(&tab, now(), 5);
        let monthly = sections
            .iter()
            .find(|projection| {
                matches!(&projection.section, Section::Metric { label, .. } if label == "Monthly")
            })
            .expect("a monthly metric");
        assert_eq!(monthly.window, Some(chrono::Duration::days(31)));
    }

    #[test]
    fn an_error_state_projects_its_message_and_no_metrics() {
        let tab = TabState::error_with_plan("boom", Some("Plan".to_string()));
        let sections = project(&tab, now(), 5);
        assert!(matches!(sections[0].section, Section::Title { .. }));
        assert!(
            sections
                .iter()
                .all(|projection| !matches!(projection.section, Section::Metric { .. }))
        );
        assert!(sections.iter().any(|projection| matches!(
            &projection.section,
            Section::Text { label, value } if label == "Error" && value == "boom"
        )));
    }

    /// Untrusted text (a provider's error body) is inert before any frontend
    /// sees it.
    #[test]
    fn sections_are_sanitized_at_the_projection_boundary() {
        let tab = TabState::error("bad\u{1b}[31mthing");
        let sections = project(&tab, now(), 5);
        assert!(sections.iter().all(|projection| match &projection.section {
            Section::Text { value, .. } => !value.contains('\u{1b}'),
            _ => true,
        }));
    }
}
