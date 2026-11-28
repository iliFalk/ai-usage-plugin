//! OpenCode Go usage history — the samples the pace chart is drawn from.
//!
//! The pace chart in the Omarchy panel plots, for every cumulative window, the
//! deviation between the reported percentage and an even burn of the window.
//! That needs more than the current reading, so each fetch appends a sample to a
//! per-window series kept in the cache. The rules are the iOS widget's
//! (`opencode-usage-widget.js`), which is the reference for the whole feature:
//!
//! - a window's series resets when `resetsAt` changes, when `resetsAt` has
//!   already passed, or when the percentage dropped by more than
//!   [`RESET_DROP_PP`] inside the same window;
//! - the drop rule applies only to the cumulative windows (week, month). The 5h
//!   rolling window's consumption legitimately falls as old requests leave it,
//!   so a drop there is not a reset;
//! - at most [`MAX_POINTS`] samples are kept.
//!
//! The cache stores only facts the provider reports (`resets_at`, points). The
//! window *start* is derived at report time from the window's length or the
//! configured billing day, because the monthly start follows configuration.

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::types::{ROLLING_WINDOW, WEEKLY_WINDOW, Window};

/// Samples kept per window — the widget's cap.
pub const MAX_POINTS: usize = 200;

/// A drop larger than this (percentage points) inside one cumulative window
/// means the window reset. Consumption inside a window only grows, so anything
/// bigger is a new window whose `resetsAt` has not moved yet.
pub const RESET_DROP_PP: f64 = 0.5;

/// One sample: milliseconds since the epoch, and the reported percentage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryPoint {
    pub t: i64,
    pub p: f64,
}

/// The samples collected inside one window.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowHistory {
    /// The provider's `resetsAt` for the window these points belong to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
    #[serde(default)]
    pub points: Vec<HistoryPoint>,
}

/// History for every window OpenCode Go reports.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageHistory {
    #[serde(default)]
    pub rolling: WindowHistory,
    #[serde(default)]
    pub weekly: WindowHistory,
    #[serde(default)]
    pub monthly: WindowHistory,
}

/// Which of the three windows a piece of history belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    Rolling,
    Weekly,
    Monthly,
}

impl WindowKind {
    /// Every window, in report order.
    pub const ALL: [Self; 3] = [Self::Rolling, Self::Weekly, Self::Monthly];

    /// Only the cumulative windows get a pace curve: the rolling one empties as
    /// requests age out, so a deviation-from-even-burn line for it is noise.
    pub const CHARTED: [Self; 2] = [Self::Weekly, Self::Monthly];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Rolling => "rolling",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }

    /// Whether consumption only grows inside a window. False for the rolling
    /// 5h window, whose percentage falls as requests leave the window.
    const fn monotone(self) -> bool {
        !matches!(self, Self::Rolling)
    }
}

impl UsageHistory {
    /// Append one sample per reported window.
    ///
    /// `now` is passed in so the reset rules are testable without a clock.
    pub fn merge(&mut self, usage: &super::types::Usage, now: DateTime<Utc>) {
        for (kind, window) in [
            (WindowKind::Rolling, usage.rolling.as_ref()),
            (WindowKind::Weekly, usage.weekly.as_ref()),
            (WindowKind::Monthly, usage.monthly.as_ref()),
        ] {
            let Some(window) = window else {
                continue;
            };
            merge_window(self.window_mut(kind), window, now, kind.monotone());
        }
    }

    fn window_mut(&mut self, kind: WindowKind) -> &mut WindowHistory {
        match kind {
            WindowKind::Rolling => &mut self.rolling,
            WindowKind::Weekly => &mut self.weekly,
            WindowKind::Monthly => &mut self.monthly,
        }
    }

    fn window(&self, kind: WindowKind) -> &WindowHistory {
        match kind {
            WindowKind::Rolling => &self.rolling,
            WindowKind::Weekly => &self.weekly,
            WindowKind::Monthly => &self.monthly,
        }
    }

    /// The report's `history` object, or `None` when no window has a usable
    /// series (no points, or a start that cannot be derived).
    ///
    /// `start` is millisecond-precise and travels with each window so the panel
    /// never re-derives window semantics: the 5h and 7d lengths are constants,
    /// and the monthly one follows `monthly_cycle_day` when configured and the
    /// widget's previous-month-same-day rule otherwise.
    pub fn report(&self, monthly_cycle_day: Option<u8>) -> Option<ReportHistory> {
        let mut windows: [Option<ReportWindow>; 3] = [None, None, None];
        for (index, kind) in WindowKind::ALL.into_iter().enumerate() {
            let history = self.window(kind);
            let Some(resets_at) = history
                .resets_at
                .as_deref()
                .and_then(|raw| raw.parse::<DateTime<chrono::FixedOffset>>().ok())
                .map(|end| end.with_timezone(&Utc))
            else {
                continue;
            };
            let Some(start) = window_start(kind, resets_at, monthly_cycle_day) else {
                continue;
            };
            if history.points.is_empty() || resets_at <= start {
                continue;
            }
            windows[index] = Some(ReportWindow {
                resets_at: resets_at.to_rfc3339(),
                start: start.timestamp_millis(),
                points: history.points.clone(),
            });
        }
        let [rolling, weekly, monthly] = windows;
        if rolling.is_none() && weekly.is_none() && monthly.is_none() {
            return None;
        }
        Some(ReportHistory {
            rolling,
            weekly,
            monthly,
        })
    }
}

fn merge_window(history: &mut WindowHistory, window: &Window, now: DateTime<Utc>, monotone: bool) {
    let resets_at = window.resets_at.to_rfc3339();
    let same_window = history.resets_at.as_deref() == Some(resets_at.as_str());
    let reset = !same_window
        || history.points.is_empty()
        || now >= window.resets_at
        || (monotone
            && history
                .points
                .last()
                .is_some_and(|last| window.percent < last.p - RESET_DROP_PP));
    if reset {
        history.points.clear();
    }
    history.resets_at = Some(resets_at);
    history.points.push(HistoryPoint {
        t: now.timestamp_millis(),
        p: window.percent,
    });
    if history.points.len() > MAX_POINTS {
        let excess = history.points.len() - MAX_POINTS;
        history.points.drain(..excess);
    }
}

/// Start of the window ending at `resets_at`, per the widget's rules.
///
/// The usage endpoint never reports a window length, so:
/// - `rolling` is the 5h limit and `weekly` the 7-day limit (constants in
///   [`super::types`], with their provenance recorded there);
/// - `monthly` follows the billing cycle: the configured cycle day in the
///   previous month when set, otherwise the same day of the previous month,
///   clamped to that month's last day (31 March → 28 February, not 3 March).
pub fn window_start(
    kind: WindowKind,
    resets_at: DateTime<Utc>,
    monthly_cycle_day: Option<u8>,
) -> Option<DateTime<Utc>> {
    match kind {
        WindowKind::Rolling => Some(resets_at - ROLLING_WINDOW),
        WindowKind::Weekly => Some(resets_at - WEEKLY_WINDOW),
        WindowKind::Monthly => {
            let day = monthly_cycle_day
                .filter(|day| (1..=31).contains(day))
                .map(u32::from)
                .unwrap_or_else(|| resets_at.day());
            previous_month_same_day(resets_at, day)
        }
    }
}

/// The same day of the previous month, clamped to that month's last day so a
/// short month cannot overflow into the next one (31 March → 28 February).
fn previous_month_same_day(end: DateTime<Utc>, day: u32) -> Option<DateTime<Utc>> {
    let (year, month) = (end.year(), end.month());
    let (previous_year, previous_month) = if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    };
    // Last day of the previous month: the day before the first of this one.
    let last_day = NaiveDate::from_ymd_opt(year, month, 1)?
        .checked_sub_days(chrono::Days::new(1))?
        .day();
    let start = NaiveDate::from_ymd_opt(previous_year, previous_month, day.min(last_day))?;
    Some(start.and_time(end.time()).and_utc())
}

/// One window of the report's `history` object.
///
/// `resetsAt` keeps the provider's own spelling rather than the document's
/// snake_case: this object mirrors the API's window record, and both the panel
/// and the widget read the name straight from it.
#[derive(Debug, Clone, Serialize)]
pub struct ReportWindow {
    #[serde(rename = "resetsAt")]
    pub resets_at: String,
    /// Window start, ms since epoch.
    pub start: i64,
    pub points: Vec<HistoryPoint>,
}

/// The report's `history` object: one entry per window that has samples.
#[derive(Debug, Clone, Serialize)]
pub struct ReportHistory {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rolling: Option<ReportWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekly: Option<ReportWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly: Option<ReportWindow>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opencode_go::types::Usage;
    use chrono::TimeZone;

    /// Weekly window ending 2026-08-20T00:00:00Z.
    fn weekly(percent: f64, resets_at: &str) -> Usage {
        let value = serde_json::json!({
            "usage": {
                "weekly": {"status": "ok", "percent": percent, "resetsAt": resets_at}
            }
        });
        super::super::types::parse_usage(&value).expect("fixture parses")
    }

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 16, hour, 0, 0).unwrap()
    }

    const RESET: &str = "2026-08-20T00:00:00Z";

    #[test]
    fn points_accumulate_inside_one_window() {
        let mut history = UsageHistory::default();
        history.merge(&weekly(10.0, RESET), at(10));
        history.merge(&weekly(12.0, RESET), at(11));
        history.merge(&weekly(14.0, RESET), at(12));

        assert_eq!(
            history.weekly.points,
            vec![
                HistoryPoint {
                    t: at(10).timestamp_millis(),
                    p: 10.0
                },
                HistoryPoint {
                    t: at(11).timestamp_millis(),
                    p: 12.0
                },
                HistoryPoint {
                    t: at(12).timestamp_millis(),
                    p: 14.0
                },
            ]
        );
    }

    #[test]
    fn a_new_resets_at_starts_a_new_series() {
        let mut history = UsageHistory::default();
        history.merge(&weekly(10.0, RESET), at(10));
        history.merge(&weekly(70.0, "2026-08-27T00:00:00Z"), at(11));

        assert_eq!(history.weekly.points.len(), 1);
        assert_eq!(history.weekly.points[0].p, 70.0);
        assert_eq!(
            history.weekly.resets_at.as_deref(),
            Some("2026-08-27T00:00:00+00:00")
        );
    }

    #[test]
    fn a_passed_resets_at_starts_a_new_series_even_when_it_does_not_move() {
        let mut history = UsageHistory::default();
        history.merge(&weekly(40.0, RESET), at(10));
        // Same `resetsAt`, but the clock has gone past it: the provider's next
        // payload has not caught up yet.
        let after_end = Utc.with_ymd_and_hms(2026, 8, 20, 1, 0, 0).unwrap();
        history.merge(&weekly(41.0, RESET), after_end);

        assert_eq!(history.weekly.points.len(), 1);
        assert_eq!(history.weekly.points[0].t, after_end.timestamp_millis());
    }

    #[test]
    fn a_drop_bigger_than_half_a_point_starts_a_new_series() {
        let mut history = UsageHistory::default();
        history.merge(&weekly(40.0, RESET), at(10));
        history.merge(&weekly(39.6, RESET), at(11));
        assert_eq!(
            history.weekly.points.len(),
            2,
            "a 0.4 pp drop is a reading, not a reset"
        );

        history.merge(&weekly(2.0, RESET), at(12));
        assert_eq!(history.weekly.points.len(), 1);
        assert_eq!(history.weekly.points[0].p, 2.0);
    }

    #[test]
    fn a_drop_below_the_threshold_keeps_the_series() {
        let mut history = UsageHistory::default();
        history.merge(&weekly(40.0, RESET), at(10));
        history.merge(&weekly(39.5, RESET), at(11));
        assert_eq!(history.weekly.points.len(), 2);
    }

    #[test]
    fn the_rolling_window_never_resets_on_a_drop() {
        let value = |percent: f64| {
            serde_json::json!({
                "usage": {
                    "rolling": {
                        "status": "ok",
                        "percent": percent,
                        "resetsAt": "2026-08-16T15:00:00Z"
                    }
                }
            })
        };
        let mut history = UsageHistory::default();
        history.merge(
            &super::super::types::parse_usage(&value(40.0)).unwrap(),
            at(10),
        );
        history.merge(
            &super::super::types::parse_usage(&value(1.0)).unwrap(),
            at(11),
        );
        assert_eq!(history.rolling.points.len(), 2);
    }

    #[test]
    fn at_most_two_hundred_points_per_window() {
        let mut history = UsageHistory::default();
        for minute in 0..(MAX_POINTS + 25) {
            let now = Utc.with_ymd_and_hms(2026, 8, 16, 0, 0, 0).unwrap()
                + chrono::Duration::minutes(minute as i64);
            // Rising inside one window, so only the cap can cut the series.
            history.merge(&weekly(minute as f64 * 0.4, RESET), now);
        }
        assert_eq!(history.weekly.points.len(), MAX_POINTS);
        // The oldest samples are the ones dropped.
        let expected_first =
            Utc.with_ymd_and_hms(2026, 8, 16, 0, 0, 0).unwrap() + chrono::Duration::minutes(25);
        assert_eq!(
            history.weekly.points[0].t,
            expected_first.timestamp_millis()
        );
    }

    #[test]
    fn the_report_carries_points_with_the_derived_start() {
        let mut history = UsageHistory::default();
        history.merge(&weekly(21.0, RESET), at(10));
        let report = history.report(None).expect("weekly series is usable");

        let weekly = report.weekly.expect("weekly window");
        assert_eq!(weekly.resets_at, "2026-08-20T00:00:00+00:00".to_string());
        // 7 days before the reset instant.
        let start = DateTime::from_timestamp_millis(weekly.start).unwrap();
        assert_eq!(start, Utc.with_ymd_and_hms(2026, 8, 13, 0, 0, 0).unwrap());
        assert_eq!(weekly.points.len(), 1);
        assert!(report.rolling.is_none());
        assert!(report.monthly.is_none());
    }

    #[test]
    fn without_points_there_is_no_history_object() {
        let history = UsageHistory::default();
        assert!(history.report(None).is_none());
    }

    #[test]
    fn an_unparsable_resets_at_is_left_out() {
        let history = UsageHistory {
            weekly: WindowHistory {
                resets_at: Some("tomorrow".into()),
                points: vec![HistoryPoint { t: 1, p: 2.0 }],
            },
            ..Default::default()
        };
        assert!(history.report(None).is_none());
    }

    #[test]
    fn the_monthly_start_uses_the_configured_cycle_day() {
        let resets = Utc.with_ymd_and_hms(2026, 9, 15, 0, 0, 0).unwrap();
        assert_eq!(
            window_start(WindowKind::Monthly, resets, Some(15)),
            Some(Utc.with_ymd_and_hms(2026, 8, 15, 0, 0, 0).unwrap())
        );
    }

    #[test]
    fn the_monthly_start_without_a_cycle_day_is_the_previous_month_same_day() {
        let end = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
        // 1 April → 1 March, not a 30-day estimate.
        assert_eq!(
            window_start(WindowKind::Monthly, end, None),
            Some(Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap())
        );
        // A reset on the 31st lands on the previous month's last day.
        let march = Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap();
        assert_eq!(
            window_start(WindowKind::Monthly, march, None),
            Some(Utc.with_ymd_and_hms(2026, 3, 30, 0, 0, 0).unwrap())
        );
        let may = Utc.with_ymd_and_hms(2026, 5, 31, 0, 0, 0).unwrap();
        assert_eq!(
            window_start(WindowKind::Monthly, may, None),
            Some(Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap())
        );
        // January rolls the year back.
        let january = Utc.with_ymd_and_hms(2026, 1, 31, 0, 0, 0).unwrap();
        assert_eq!(
            window_start(WindowKind::Monthly, january, None),
            Some(Utc.with_ymd_and_hms(2025, 12, 31, 0, 0, 0).unwrap())
        );
    }
}
