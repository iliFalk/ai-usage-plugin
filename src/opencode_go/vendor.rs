use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::config::OpenCodeGoConfig;
use crate::countdown;
use crate::format::{placeholders, substitute, updated_at_hm};
use crate::pacing::{self, PaceSeverity};
use crate::pango::{color_span, escape, severity_color, severity_for};
use crate::theme::Theme;
use crate::tooltip::{Line as TooltipLine, WindowRow, push_window_with_row, render_bordered};
use crate::usage::UsageWindow;
use crate::vendor::{RenderOpts, VendorId, VendorOutcome};
use crate::waybar::{Class, WaybarOutput};

use super::fetch::FetchOutcome;
use super::types::{Usage, Window};

pub const DEFAULT_FORMAT: &str = "{ocg_rolling_pct}% · {ocg_rolling_reset}";
const DEFAULT_PLAN: &str = "OpenCode Go";
const UNAVAILABLE: &str = "—";

/// Dollar limits for the three OpenCode Go windows. Defaults mirror the
/// Scriptable widget: rolling 5h → $15, weekly → $40, monthly → $80.
#[derive(Debug, Clone, Default)]
pub struct OpenCodeGoRenderConfig {
    pub month_start_ms: Option<i64>,
    pub month_days: Option<u32>,
    pub rolling_limit: f64,
    pub weekly_limit: f64,
    pub monthly_limit: f64,
}

impl OpenCodeGoRenderConfig {
    pub fn from_config(cfg: &OpenCodeGoConfig) -> Self {
        Self {
            month_start_ms: cfg.month_start_ms,
            month_days: cfg.month_days,
            rolling_limit: cfg.rolling_limit.unwrap_or(15.0),
            weekly_limit: cfg.weekly_limit.unwrap_or(40.0),
            monthly_limit: cfg.monthly_limit.unwrap_or(80.0),
        }
    }
}

/// Start of a usage window in ms since the epoch.
///
/// - rolling: `resets_at - 5h`
/// - weekly:  `resets_at - 7d`
/// - monthly: `month_start_ms` > `month_days` back from reset > calendar month
pub fn window_start_ms(
    kind: &str,
    resets_at: DateTime<Utc>,
    cfg: &OpenCodeGoRenderConfig,
) -> Option<i64> {
    let end_ms = resets_at.timestamp_millis();
    match kind {
        "rolling" => Some(end_ms - ROLLING_WINDOW.num_milliseconds()),
        "weekly" => Some(end_ms - WEEKLY_WINDOW.num_milliseconds()),
        "monthly" => {
            if let Some(start) = cfg.month_start_ms {
                return Some(start);
            }
            if let Some(days) = cfg.month_days {
                return Some(end_ms - (days as i64) * 86_400_000);
            }
            // No configured cycle start: monthly pacing is unavailable
            // (subscriber-dependent cycle length would make any guess wrong).
            None
        }
        _ => None,
    }
}

/// Difference between actual percent and linear pace (percent points).
/// Positive = ahead of linear pace, negative = under.
fn pace_diff(percent: f64, start_ms: i64, end_ms: i64, now_ms: i64) -> Option<f64> {
    if end_ms <= start_ms {
        return None;
    }
    let elapsed = ((now_ms - start_ms) as f64) / ((end_ms - start_ms) as f64);
    let elapsed = elapsed.clamp(0.0, 1.0);
    Some(percent - elapsed * 100.0)
}

fn pace_diff_text(pace: Option<f64>) -> (String, &'static str) {
    match pace {
        None => ("—".into(), "gray"),
        Some(p) if p.abs() < 0.5 => ("genau im Plan".into(), "gray"),
        Some(p) if p > 0.0 => {
            let color = if p >= 20.0 { "red" } else { "orange" };
            (format!("+{:.0}% über linear", p), color)
        }
        Some(p) => {
            let color = "green";
            (format!("{:.0}% unter linear", p), color)
        }
    }
}

/// Window lengths for pacing math. The usage endpoint reports only `status`,
/// `percent`, and `resetsAt` — never a duration — so these are constants with
/// a recorded provenance:
/// - `rolling` is the 5-hour limit (`packages/console/app/src/routes/zen/go/v1/usage.ts`
///   `formatUsage` + `packages/console/app/src/routes/zen/util/handler.ts` and
///   `i18n/en.ts` "5-hour usage limit reached", validated live via
///   `GET https://opencode.ai/zen/go/v1/usage`).
/// - `weekly` resets Monday 00:00 UTC (7 days, same response capture).
///
/// There is deliberately no monthly constant: the monthly window follows the
/// subscription cycle (`getMonthlyBounds(now, timeSubscribed)` upstream —
/// 28/29/31-day months depending on the subscriber), so any fixed length
/// would pace against a wrong denominator and publish a wrong `window_secs`.
/// The monthly reset is still shown; only pacing is omitted until the API
/// reports real cycle bounds.
pub const ROLLING_WINDOW: chrono::Duration = chrono::Duration::hours(5);
pub const WEEKLY_WINDOW: chrono::Duration = chrono::Duration::days(7);

impl From<FetchOutcome> for VendorOutcome {
    fn from(outcome: FetchOutcome) -> Self {
        outcome.map(crate::usage::VendorSnapshot::OpenCodeGo)
    }
}

/// Build placeholders with the historical default pacing tolerance.
///
/// Keep this signature stable for library callers. Rendering uses the private
/// tolerance-aware helper so `--pace-tolerance` still applies.
pub fn build_placeholders(usage: &Usage, now: DateTime<Utc>) -> HashMap<&'static str, String> {
    let cfg = OpenCodeGoRenderConfig::default();
    build_placeholders_with_plan(DEFAULT_PLAN, usage, now, &cfg)
}

pub fn build_placeholders_with_plan(
    plan: &str,
    usage: &Usage,
    now: DateTime<Utc>,
    cfg: &OpenCodeGoRenderConfig,
) -> HashMap<&'static str, String> {
    build_placeholders_with_plan_and_tolerance(plan, usage, pacing::DEFAULT_TOLERANCE, now, cfg)
}

fn build_placeholders_with_tolerance(
    usage: &Usage,
    pace_tolerance: u32,
    now: DateTime<Utc>,
    cfg: &OpenCodeGoRenderConfig,
) -> HashMap<&'static str, String> {
    build_placeholders_with_plan_and_tolerance(DEFAULT_PLAN, usage, pace_tolerance, now, cfg)
}

fn build_placeholders_with_plan_and_tolerance(
    plan: &str,
    usage: &Usage,
    pace_tolerance: u32,
    now: DateTime<Utc>,
    cfg: &OpenCodeGoRenderConfig,
) -> HashMap<&'static str, String> {
    let plan = sanitize(plan);
    let rolling = window_values(usage.rolling.as_ref(), now);
    let weekly = window_values(usage.weekly.as_ref(), now);
    let monthly = window_values(usage.monthly.as_ref(), now);
    let _now_ms = now.timestamp_millis();
    let rolling_pace = window_pacing(
        usage.rolling.as_ref(),
        "rolling",
        ROLLING_WINDOW,
        pace_tolerance,
        now,
        cfg,
    );
    let weekly_pace = window_pacing(
        usage.weekly.as_ref(),
        "weekly",
        WEEKLY_WINDOW,
        pace_tolerance,
        now,
        cfg,
    );
    let monthly_pace = window_pacing(
        usage.monthly.as_ref(),
        "monthly",
        chrono::Duration::zero(), // unused for monthly; start comes from cfg
        pace_tolerance,
        now,
        cfg,
    );

    // Dollar amounts: percent * limit / 100
    let rolling_usd = rolling.percent.parse::<f64>().unwrap_or(0.0) / 100.0 * cfg.rolling_limit;
    let weekly_usd = weekly.percent.parse::<f64>().unwrap_or(0.0) / 100.0 * cfg.weekly_limit;
    let monthly_usd = monthly.percent.parse::<f64>().unwrap_or(0.0) / 100.0 * cfg.monthly_limit;

    let mut vals: Vec<(&'static str, String)> = vec![
        ("icon", VendorId::OpenCodeGo.bar_icon().to_string()),
        (
            "vendor_short",
            VendorId::OpenCodeGo.short_name().to_string(),
        ),
        ("plan", plan.clone()),
        ("ocg_plan", plan),
        ("session_pct", rolling.percent.clone()),
        ("session_reset", rolling.reset.clone()),
        ("session_elapsed", rolling_pace.elapsed.clone()),
        ("weekly_pct", weekly.percent.clone()),
        ("weekly_reset", weekly.reset.clone()),
        ("weekly_elapsed", weekly_pace.elapsed.clone()),
        ("ocg_rolling_pct", rolling.percent),
        ("ocg_rolling_reset", rolling.reset),
        ("ocg_rolling_status", rolling.status),
        ("ocg_rolling_elapsed", rolling_pace.elapsed),
        ("ocg_rolling_pace", rolling_pace.ratio_pace),
        ("ocg_rolling_pace_indicator", rolling_pace.point_pace),
        ("ocg_weekly_pct", weekly.percent),
        ("ocg_weekly_reset", weekly.reset),
        ("ocg_weekly_status", weekly.status),
        ("ocg_weekly_elapsed", weekly_pace.elapsed),
        ("ocg_weekly_pace", weekly_pace.ratio_pace),
        ("ocg_weekly_pace_indicator", weekly_pace.point_pace),
        ("ocg_monthly_pct", monthly.percent),
        ("ocg_monthly_reset", monthly.reset),
        ("ocg_monthly_status", monthly.status),
        // Dollar amounts.
        ("ocg_rolling_usd", format!("{:.2}", rolling_usd)),
        ("ocg_weekly_usd", format!("{:.2}", weekly_usd)),
        ("ocg_monthly_usd", format!("{:.2}", monthly_usd)),
        ("ocg_rolling_limit", format!("{:.2}", cfg.rolling_limit)),
        ("ocg_weekly_limit", format!("{:.2}", cfg.weekly_limit)),
        ("ocg_monthly_limit", format!("{:.2}", cfg.monthly_limit)),
        // Pace diff (percent points ahead/behind linear).
        (
            "ocg_rolling_pace_diff",
            rolling_pace
                .pace_diff
                .map_or_else(|| "—".into(), |p| format!("{p:.0}")),
        ),
        (
            "ocg_weekly_pace_diff",
            weekly_pace
                .pace_diff
                .map_or_else(|| "—".into(), |p| format!("{p:.0}")),
        ),
        (
            "ocg_monthly_pace_diff",
            monthly_pace
                .pace_diff
                .map_or_else(|| "—".into(), |p| format!("{p:.0}")),
        ),
        // Pace diff text with color hint (used by tooltip).
        (
            "ocg_rolling_pace_text",
            rolling_pace.pace_diff_text.0.clone(),
        ),
        ("ocg_weekly_pace_text", weekly_pace.pace_diff_text.0.clone()),
        (
            "ocg_monthly_pace_text",
            monthly_pace.pace_diff_text.0.clone(),
        ),
    ];

    // Monthly pacing is only available when a cycle start is configured.
    if cfg.month_start_ms.is_some() || cfg.month_days.is_some() {
        vals.push(("ocg_monthly_elapsed", monthly_pace.elapsed.clone()));
        vals.push(("ocg_monthly_pace", monthly_pace.ratio_pace.clone()));
        vals.push((
            "ocg_monthly_pace_indicator",
            monthly_pace.point_pace.clone(),
        ));
    } else {
        vals.push(("ocg_monthly_elapsed", String::new()));
        vals.push(("ocg_monthly_pace", String::new()));
        vals.push(("ocg_monthly_pace_indicator", String::new()));
    }

    placeholders(vals)
}

/// Elapsed fraction, pace glyphs, and pace-diff for one window.
#[derive(Default)]
struct WindowPacing {
    elapsed: String,
    ratio_pace: String,
    point_pace: String,
    /// Pace diff in percent points (actual − linear). `None` when the window
    /// start cannot be determined (e.g. monthly without a configured cycle
    /// start).
    pace_diff: Option<f64>,
    /// Formatted pace text + color hint, e.g. ("+15% über linear", "orange").
    pace_diff_text: (String, &'static str),
}

fn window_pacing(
    window: Option<&Window>,
    kind: &str,
    duration: chrono::Duration,
    pace_tolerance: u32,
    now: DateTime<Utc>,
    cfg: &OpenCodeGoRenderConfig,
) -> WindowPacing {
    let Some(window) = window else {
        return WindowPacing::default();
    };
    let p = pacing::calc(
        utilization_pct(window),
        Some(window.resets_at),
        now,
        duration,
        pace_tolerance,
    );

    // Pace diff: actual % minus linear-elapsed %. Only computable when we
    // know the window start.
    let (pace_diff, pace_diff_text) = match window_start_ms(kind, window.resets_at, cfg) {
        Some(start_ms) => {
            let end_ms = window.resets_at.timestamp_millis();
            let now_ms = now.timestamp_millis();
            let pd = pace_diff(window.percent, start_ms, end_ms, now_ms);
            let text = pace_diff_text(pd);
            (pd, text)
        }
        None => (None, ("—".into(), "gray")),
    };

    WindowPacing {
        elapsed: p.elapsed_pct.to_string(),
        ratio_pace: p.ratio_pace.glyph().to_string(),
        point_pace: p.point_pace.glyph().to_string(),
        pace_diff,
        pace_diff_text: (pace_diff_text.0, pace_diff_text.1),
    }
}

/// Project an OpenCode Go window onto the shared shape the tooltip helper
/// draws. Percentages arrive as floats; panels round them like every other
/// vendor's integer-percent convention.
fn as_usage_window(window: &Window, duration: chrono::Duration) -> UsageWindow {
    UsageWindow {
        utilization_pct: utilization_pct(window),
        resets_at: Some(window.resets_at),
        window_duration: duration,
    }
}

fn utilization_pct(window: &Window) -> i32 {
    i32::from(crate::format::clamp_pct(window.percent))
}

#[derive(Debug)]
struct WindowValues {
    percent: String,
    reset: String,
    status: String,
}

fn window_values(window: Option<&Window>, now: DateTime<Utc>) -> WindowValues {
    let Some(window) = window else {
        return WindowValues {
            percent: UNAVAILABLE.to_string(),
            reset: UNAVAILABLE.to_string(),
            status: UNAVAILABLE.to_string(),
        };
    };
    WindowValues {
        percent: window.percent.to_string(),
        reset: countdown::format(Some(window.resets_at), now),
        status: sanitize(&window.status),
    }
}

fn sanitize(value: &str) -> String {
    crate::display::sanitize_untrusted_field(value)
}

pub fn severity(usage: &Usage) -> PaceSeverity {
    usage
        .rolling
        .iter()
        .chain(usage.weekly.iter())
        .chain(usage.monthly.iter())
        .map(|window| window.percent as i32)
        .max()
        .map(severity_for)
        .unwrap_or(PaceSeverity::Low)
}

/// Renderer shape matches the existing vendor adapters. `snap` remains local
/// because the shared enum does not yet have an OpenCode-Go arm.
pub fn render(
    outcome: &VendorOutcome,
    snap: &Usage,
    theme: &Theme,
    opts: &RenderOpts,
    cfg: &OpenCodeGoRenderConfig,
    now: DateTime<Utc>,
    history: &crate::opencode_go::fetch::UsageHistory,
) -> WaybarOutput {
    render_with_meta(
        snap,
        outcome.stale,
        outcome.last_error.as_ref(),
        outcome.cache_age,
        theme,
        opts,
        cfg,
        now,
        history,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_with_meta(
    snap: &Usage,
    stale: bool,
    last_error: Option<&(u16, String)>,
    cache_age: Option<Duration>,
    theme: &Theme,
    opts: &RenderOpts,
    cfg: &OpenCodeGoRenderConfig,
    now: DateTime<Utc>,
    history: &crate::opencode_go::fetch::UsageHistory,
) -> WaybarOutput {
    let sev = severity(snap);
    let format = opts.format.as_deref().unwrap_or(DEFAULT_FORMAT);
    let values = escaped_placeholders_with_tolerance(snap, opts.pace_tolerance, now, cfg);
    let mut text = substitute(format, &values);
    if stale {
        text.push_str(" ⏸");
    }
    let icon_prefix = match opts.icon.as_deref() {
        Some(icon) if !icon.is_empty() => format!("{} ", escape(icon)),
        _ => String::new(),
    };
    let bar_text = color_span(severity_color(sev, theme), &format!("{icon_prefix}{text}"));
    let tooltip = opts
        .tooltip_format
        .as_deref()
        .map(|format| substitute(format, &values))
        .unwrap_or_else(|| {
            render_tooltip(
                snap, stale, last_error, cache_age, theme, opts, cfg, now, history,
            )
        });

    WaybarOutput {
        text: bar_text,
        tooltip,
        class: Class::from(sev),
    }
}

fn escaped_placeholders_with_tolerance(
    usage: &Usage,
    pace_tolerance: u32,
    now: DateTime<Utc>,
    cfg: &OpenCodeGoRenderConfig,
) -> HashMap<&'static str, String> {
    let mut values = build_placeholders_with_tolerance(usage, pace_tolerance, now, cfg);
    for key in [
        "plan",
        "ocg_plan",
        "ocg_rolling_status",
        "ocg_weekly_status",
        "ocg_monthly_status",
    ] {
        if let Some(value) = values.get_mut(key) {
            *value = escape(value);
        }
    }
    values
}

/// Unicode block sparkline: converts a series of percentages into block
/// characters (▁▂▃▄▅▆▇█) scaled to the max value in the series.
fn sparkline(points: &[f64], width: usize) -> String {
    if points.is_empty() {
        return String::new();
    }
    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max_p = points.iter().copied().fold(0.0f64, f64::max).max(1.0);
    let step = points.len().div_euclid(width.max(1));
    let mut out = String::with_capacity(width);
    for i in 0..width {
        let src = i * step;
        let p = points.get(src).copied().unwrap_or(0.0);
        let idx = ((p / max_p) * (blocks.len() - 1) as f64).round() as usize;
        out.push(blocks[idx.min(blocks.len() - 1)]);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn render_tooltip(
    snap: &Usage,
    stale: bool,
    last_error: Option<&(u16, String)>,
    cache_age: Option<Duration>,
    theme: &Theme,
    opts: &RenderOpts,
    cfg: &OpenCodeGoRenderConfig,
    now: DateTime<Utc>,
    history: &crate::opencode_go::fetch::UsageHistory,
) -> String {
    let mut lines = vec![TooltipLine::Center(format!(
        "<span font_weight='bold' foreground='{}'>{}</span>",
        theme.blue,
        escape(DEFAULT_PLAN)
    ))];
    lines.push(TooltipLine::Sep);
    lines.push(TooltipLine::Body(String::new()));

    let row =
        |w: &UsageWindow| WindowRow::paced(w, now, opts.pace_tolerance, opts.tooltip_pace_pts);
    let mut present = false;
    for (label, window, duration, kind) in [
        (
            "  Rolling (5h)",
            snap.rolling.as_ref(),
            ROLLING_WINDOW,
            "rolling",
        ),
        (
            "  Weekly (7d)",
            snap.weekly.as_ref(),
            WEEKLY_WINDOW,
            "weekly",
        ),
    ] {
        let Some(window) = window else {
            continue;
        };
        present = true;
        let projected = as_usage_window(window, duration);
        push_window_with_row(&mut lines, label, &projected, theme, now, row(&projected));
        // Dollar line + pace diff below each window row.
        let pct = window.percent;
        let usd = pct / 100.0
            * match kind {
                "rolling" => cfg.rolling_limit,
                "weekly" => cfg.weekly_limit,
                _ => 0.0,
            };
        let limit = match kind {
            "rolling" => cfg.rolling_limit,
            "weekly" => cfg.weekly_limit,
            _ => 0.0,
        };
        let start_ms = window_start_ms(kind, window.resets_at, cfg);
        let pace = start_ms.map(|s| {
            let end_ms = window.resets_at.timestamp_millis();
            let now_ms = now.timestamp_millis();
            pace_diff(pct, s, end_ms, now_ms)
        });
        let (pace_text, pace_color) = match pace {
            Some(Some(p)) => {
                let (text, color) = pace_diff_text(Some(p));
                (
                    text,
                    match color {
                        "red" => theme.red.clone(),
                        "orange" => theme.orange.clone(),
                        "green" => theme.green.clone(),
                        _ => theme.dim.clone(),
                    },
                )
            }
            _ => ("—".into(), theme.dim.clone()),
        };
        lines.push(TooltipLine::Body(format!(
            " <span foreground='{}'>    ${:.2} / ${:.2}</span>",
            theme.fg, usd, limit
        )));
        lines.push(TooltipLine::Body(format!(
            " <span foreground='{}'>    pace: {}</span>",
            pace_color,
            escape(&pace_text)
        )));
        // Sparkline: usage trend over the window.
        let hist = match kind {
            "rolling" => &history.rolling,
            "weekly" => &history.weekly,
            _ => &history.monthly,
        };
        if !hist.points.is_empty() {
            let pts: Vec<f64> = hist.points.iter().map(|pt| pt.p).collect();
            let line = sparkline(&pts, 28);
            lines.push(TooltipLine::Body(format!(
                " <span foreground='{}'>    {}</span>",
                theme.dim, line
            )));
        }
    }
    // Monthly: pacing available when cycle start configured.
    if let Some(window) = snap.monthly.as_ref() {
        present = true;
        let pct = utilization_pct(window);
        let color = severity_color(severity_for(pct), theme);
        let bar = crate::pango::progress_bar(pct, color, theme, None);
        lines.push(TooltipLine::Body(format!(
            " <span foreground='{}'>  Monthly</span>",
            theme.fg
        )));
        lines.push(TooltipLine::Body(format!(
            "   {bar}  <span font_weight='bold' foreground='{color}'>{pct}%</span>"
        )));
        let usd = pct as f64 / 100.0 * cfg.monthly_limit;
        lines.push(TooltipLine::Body(format!(
            " <span foreground='{}'>    ${:.2} / ${:.2}</span>",
            theme.fg, usd, cfg.monthly_limit
        )));
        let pace_start = window_start_ms("monthly", window.resets_at, cfg);
        let (pace_text, pace_color) = match pace_start {
            Some(s) => {
                let end_ms = window.resets_at.timestamp_millis();
                let now_ms = now.timestamp_millis();
                let pd = pace_diff(pct as f64, s, end_ms, now_ms);
                let (text, color) = pace_diff_text(pd);
                (
                    text,
                    match color {
                        "red" => theme.red.clone(),
                        "orange" => theme.orange.clone(),
                        "green" => theme.green.clone(),
                        _ => theme.dim.clone(),
                    },
                )
            }
            None => ("—".into(), theme.dim.clone()),
        };
        lines.push(TooltipLine::Body(format!(
            " <span foreground='{}'>    pace: {}</span>",
            pace_color,
            escape(&pace_text)
        )));
        // Sparkline: usage trend over the month.
        if !history.monthly.points.is_empty() {
            let pts: Vec<f64> = history.monthly.points.iter().map(|pt| pt.p).collect();
            let line = sparkline(&pts, 28);
            lines.push(TooltipLine::Body(format!(
                " <span foreground='{}'>    {}</span>",
                theme.dim, line
            )));
        }
        lines.push(TooltipLine::Body(format!(
            " <span foreground='{}'>  ⏱  Resets in {}</span>",
            theme.dim.clone(),
            escape(&countdown::format(Some(window.resets_at), now))
        )));
    }
    if !present {
        lines.push(TooltipLine::Body(format!(
            " <span foreground='{}'>no usage windows reported</span>",
            theme.dim
        )));
    }
    if stale {
        lines.push(TooltipLine::Body(String::new()));
        lines.push(TooltipLine::Body(format!(
            " <span foreground='{}'>  ⏸  Showing cached data</span>",
            theme.orange
        )));
    }
    if let Some((code, message)) = last_error
        && *code != 0
    {
        lines.push(TooltipLine::Body(String::new()));
        lines.push(TooltipLine::Sep);
        lines.push(TooltipLine::Body(format!(
            " <span foreground='{}'>  HTTP {code}: {}</span>",
            theme.orange,
            escape(message)
        )));
    }

    lines.push(TooltipLine::Body(String::new()));
    lines.push(TooltipLine::Sep);
    lines.push(TooltipLine::Body(format!(
        " <span foreground='{}'>  Updated {}</span>",
        theme.dim,
        updated_at_hm(now, cache_age)
    )));
    render_bordered(&lines, theme)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::opencode_go::types::{Usage, Window};

    fn at(value: &str) -> DateTime<Utc> {
        value.parse().expect("RFC3339 timestamp")
    }

    fn sample_usage() -> Usage {
        Usage {
            rolling: Some(Window {
                status: "ok".into(),
                percent: 12.3,
                resets_at: at("2026-08-16T20:00:00Z"),
            }),
            weekly: Some(Window {
                status: "rate-limited".into(),
                percent: 45.6,
                resets_at: at("2026-08-20T00:00:00Z"),
            }),
            monthly: Some(Window {
                status: "ok".into(),
                percent: 78.9,
                resets_at: at("2026-09-01T00:00:00Z"),
            }),
        }
    }

    #[test]
    fn exposes_exact_opencode_go_and_generic_placeholders() {
        let values = build_placeholders(&sample_usage(), at("2026-08-16T18:00:00Z"));

        assert_eq!(values["icon"], "ocg");
        assert_eq!(values["vendor_short"], "ocg");
        assert_eq!(values["session_pct"], "12.3");
        assert_eq!(values["weekly_pct"], "45.6");
        assert_eq!(values["ocg_rolling_pct"], "12.3");
        assert_eq!(values["ocg_rolling_status"], "ok");
        assert_eq!(values["ocg_weekly_pct"], "45.6");
        assert_eq!(values["ocg_weekly_status"], "rate-limited");
        assert_eq!(values["ocg_monthly_pct"], "78.9");
        assert_eq!(values["ocg_monthly_status"], "ok");
    }

    #[test]
    fn default_format_is_rolling_percentage_and_reset() {
        assert_eq!(DEFAULT_FORMAT, "{ocg_rolling_pct}% · {ocg_rolling_reset}");
    }

    #[test]
    fn absent_windows_are_unavailable_not_zero() {
        let values = build_placeholders(
            &Usage {
                rolling: None,
                weekly: None,
                monthly: None,
            },
            at("2026-08-16T18:00:00Z"),
        );

        for key in [
            "session_pct",
            "weekly_pct",
            "ocg_rolling_pct",
            "ocg_weekly_pct",
            "ocg_monthly_pct",
        ] {
            assert_eq!(values[key], "—", "{key} should be unavailable");
            assert_ne!(values[key], "0");
        }
        for key in [
            "session_reset",
            "weekly_reset",
            "ocg_rolling_reset",
            "ocg_weekly_reset",
            "ocg_monthly_reset",
            "ocg_rolling_status",
            "ocg_weekly_status",
            "ocg_monthly_status",
        ] {
            assert_eq!(values[key], "—", "{key} should be unavailable");
        }
    }

    fn opts() -> RenderOpts {
        RenderOpts {
            format: None,
            tooltip_format: None,
            icon: None,
            pace_tolerance: 5,
            format_pace_color: false,
            tooltip_pace_pts: false,
        }
    }

    fn outcome_for(snap: &Usage) -> VendorOutcome {
        VendorOutcome {
            snapshot: crate::usage::VendorSnapshot::OpenCodeGo(snap.clone()),
            stale: false,
            last_error: None,
            cache_age: Some(std::time::Duration::from_secs(10)),
        }
    }

    #[test]
    fn elapsed_placeholders_follow_window_progress() {
        // 2h left of a 5h window → 60% elapsed; 3d left of a 7d window → 57%.
        let now = at("2026-08-16T18:00:00Z");
        let usage = Usage {
            rolling: Some(Window {
                status: "ok".into(),
                percent: 42.0,
                resets_at: at("2026-08-16T20:00:00Z"),
            }),
            weekly: Some(Window {
                status: "ok".into(),
                percent: 15.0,
                resets_at: at("2026-08-19T18:00:00Z"),
            }),
            monthly: Some(Window {
                status: "ok".into(),
                percent: 70.0,
                resets_at: at("2026-08-31T18:00:00Z"),
            }),
        };
        let values = build_placeholders(&usage, now);
        assert_eq!(values["session_elapsed"], "60");
        assert_eq!(values["weekly_elapsed"], "57");
        assert_eq!(values["ocg_rolling_elapsed"], "60");
        assert_eq!(values["ocg_weekly_elapsed"], "57");
    }

    #[test]
    fn pace_placeholders_follow_usage_vs_elapsed() {
        // Rolling: 80% used vs 60% elapsed → ahead. Weekly: 15% vs 57% → under.
        let now = at("2026-08-16T18:00:00Z");
        let usage = Usage {
            rolling: Some(Window {
                status: "ok".into(),
                percent: 80.0,
                resets_at: at("2026-08-16T20:00:00Z"),
            }),
            weekly: Some(Window {
                status: "ok".into(),
                percent: 15.0,
                resets_at: at("2026-08-19T18:00:00Z"),
            }),
            monthly: Some(Window {
                status: "ok".into(),
                percent: 70.0,
                resets_at: at("2026-08-31T18:00:00Z"),
            }),
        };
        let values = build_placeholders(&usage, now);
        assert_eq!(values["ocg_rolling_pace"], "↑");
        assert_eq!(values["ocg_rolling_pace_indicator"], "↑");
        assert_eq!(values["ocg_weekly_pace"], "↓");
        assert_eq!(values["ocg_weekly_pace_indicator"], "↓");
    }

    #[test]
    fn monthly_keeps_reset_but_omits_pacing_until_cycle_bounds_are_known() {
        // The monthly cycle is subscriber-dependent (28/29/31 days), so no
        // fixed denominator may pace it and no `window_secs` may describe it.
        // The reset countdown and status still report.
        let now = at("2026-08-16T18:00:00Z");
        let usage = Usage {
            rolling: None,
            weekly: None,
            monthly: Some(Window {
                status: "ok".into(),
                percent: 70.0,
                resets_at: at("2026-08-31T18:00:00Z"),
            }),
        };
        let values = build_placeholders(&usage, now);
        assert_eq!(values["ocg_monthly_pct"], "70");
        assert_eq!(values["ocg_monthly_reset"], "15d 0h");
        assert_eq!(values["ocg_monthly_status"], "ok");
        assert_eq!(values["ocg_monthly_elapsed"], "");
        assert_eq!(values["ocg_monthly_pace"], "");
        assert_eq!(values["ocg_monthly_pace_indicator"], "");
    }

    fn cfg() -> OpenCodeGoRenderConfig {
        OpenCodeGoRenderConfig::default()
    }

    #[test]
    fn public_builder_keeps_the_default_tolerance_api() {
        let now = at("2026-08-16T18:00:00Z");
        let usage = sample_usage();
        assert_eq!(
            build_placeholders(&usage, now),
            build_placeholders_with_tolerance(&usage, pacing::DEFAULT_TOLERANCE, now, &cfg())
        );
    }

    #[test]
    fn custom_tolerance_changes_the_ratio_pace() {
        // 53% used vs 50% elapsed → ratio 106%: ahead at ±5, on track at ±10.
        let now = at("2026-08-16T18:00:00Z");
        let usage = Usage {
            rolling: Some(Window {
                status: "ok".into(),
                percent: 53.0,
                resets_at: at("2026-08-16T20:30:00Z"),
            }),
            weekly: None,
            monthly: None,
        };
        assert_eq!(
            build_placeholders_with_tolerance(&usage, 5, now, &cfg())["ocg_rolling_pace"],
            "↑"
        );
        assert_eq!(
            build_placeholders_with_tolerance(&usage, 10, now, &cfg())["ocg_rolling_pace"],
            "→"
        );
    }

    #[test]
    fn elapsed_and_pace_placeholders_are_empty_without_window() {
        let values = build_placeholders(
            &Usage {
                rolling: None,
                weekly: None,
                monthly: None,
            },
            at("2026-08-16T18:00:00Z"),
        );
        for key in [
            "session_elapsed",
            "weekly_elapsed",
            "ocg_rolling_elapsed",
            "ocg_rolling_pace",
            "ocg_rolling_pace_indicator",
            "ocg_weekly_elapsed",
            "ocg_weekly_pace",
            "ocg_weekly_pace_indicator",
            "ocg_monthly_elapsed",
            "ocg_monthly_pace",
            "ocg_monthly_pace_indicator",
        ] {
            assert_eq!(values[key], "", "{key} should be empty");
        }
    }

    #[test]
    fn tooltip_shows_the_pace_arrow_next_to_each_percentage() {
        // Rolling: 80% used with 2h left of 5h → 60% elapsed → ahead.
        let now = at("2026-08-16T18:00:00Z");
        let usage = Usage {
            rolling: Some(Window {
                status: "ok".into(),
                percent: 80.0,
                resets_at: at("2026-08-16T20:00:00Z"),
            }),
            weekly: None,
            monthly: None,
        };
        let out = render(
            &outcome_for(&usage),
            &usage,
            &Theme::default(),
            &opts(),
            &cfg(),
            now,
            &crate::opencode_go::fetch::UsageHistory::default(),
        );
        assert!(out.tooltip.contains("80% ↑"), "{}", out.tooltip);
    }

    #[test]
    fn tooltip_monthly_row_keeps_reset_without_a_pace_glyph() {
        let now = at("2026-08-16T18:00:00Z");
        let usage = Usage {
            rolling: None,
            weekly: None,
            monthly: Some(Window {
                status: "ok".into(),
                percent: 70.0,
                resets_at: at("2026-08-31T18:00:00Z"),
            }),
        };
        let out = render(
            &outcome_for(&usage),
            &usage,
            &Theme::default(),
            &opts(),
            &cfg(),
            now,
            &crate::opencode_go::fetch::UsageHistory::default(),
        );
        assert!(out.tooltip.contains("Monthly"), "{}", out.tooltip);
        assert!(out.tooltip.contains("70%"), "{}", out.tooltip);
        assert!(
            !out.tooltip.contains('↑') && !out.tooltip.contains('→') && !out.tooltip.contains('↓'),
            "monthly row must not grow a pace glyph: {}",
            out.tooltip
        );
        assert!(out.tooltip.contains("Resets in 15d 0h"), "{}", out.tooltip);
    }

    #[test]
    fn plan_and_status_are_sanitized() {
        let usage = Usage {
            rolling: Some(Window {
                status: "ok\u{1b}[31m\u{7}".into(),
                percent: 1.0,
                resets_at: at("2026-08-16T20:00:00Z"),
            }),
            weekly: None,
            monthly: None,
        };
        let values = build_placeholders_with_plan(
            "OpenCode\u{1b}[31m Go",
            &usage,
            at("2026-08-16T18:00:00Z"),
            &cfg(),
        );

        assert!(!values["plan"].contains('\u{1b}'));
        assert!(!values["ocg_rolling_status"].contains('\u{1b}'));
        assert!(!values["ocg_rolling_status"].contains('\u{7}'));
    }
}
