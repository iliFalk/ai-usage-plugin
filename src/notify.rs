//! Quota-threshold desktop notifications.
//!
//! After a *fresh* fetch (never a cached, stale, or failed one — see
//! [`crate::outcome::Outcome::off_the_wire`]), the metric rows a refresh
//! produced are checked against a configurable threshold. A window that
//! crosses the threshold raises one notification per crossing.
//!
//! Layout:
//! - [`decide`] is the pure decision function: rows + threshold + previous
//!   state + now → notifications, with two-band urgency and hysteresis
//!   re-arming.
//! - [`NotifyState`] is the persisted dedupe map
//!   (`~/.cache/ai-usagebar/notifications.json`), written atomically under
//!   the same flock discipline as the vendor caches.
//! - [`NotifySink`] delivers through `notify-send`, without changing the
//!   decision or persisted state.
//!
//! Everything here is best-effort: a missing notifier, an unwritable state
//! file, or a contended lock is a silent skip — a missed notification beats
//! a double one, and nothing in this module may change a caller's return
//! value or exit code.
//!
//! No burn-rate extrapolation anywhere: reset times in bodies are only the
//! vendor-reported absolute instants.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::countdown;
use crate::display::sanitize_untrusted_line;
use crate::format;

/// Hysteresis band, in percentage points. A notified key re-arms only when
/// usage drops `threshold - 7` or further, so a window hovering at the
/// threshold (97 → 96 → 97) does not re-fire on every refresh.
const HYSTERESIS_PCT: i32 = 7;

/// How much later a reported reset instant has to be before it counts as a new
/// window. Vendors report that instant with sub-second precision that jitters
/// between fetches — a five-hour window came back 0.7s apart on two
/// fetches four minutes apart — and a rolling window slides its reset forward
/// with the refresh interval, so a strict comparison re-armed the key and
/// re-notified on refresh with nothing changed. Ninety minutes clears every
/// refresh interval this ships with (the bar's default is five minutes and its
/// maximum an hour; the panel refreshes every five minutes at most) and stays far below
/// the shortest window, so a real new window still moves the instant by more
/// than this.
const RESET_MOVE_TOLERANCE_SECS: i64 = 90 * 60;

/// How long to wait for the state lock before skipping the whole check.
/// Deliberately short: notifications are never worth stalling a refresh for.
const LOCK_TIMEOUT: Duration = Duration::from_secs(2);

/// A notifier that hangs must not hang the bar with it.
#[cfg(target_os = "linux")]
const SPAWN_KILL_AFTER: Duration = Duration::from_secs(5);

/// Timeout a critical toast asks `notify-send` for, in milliseconds.
///
/// Omarchy's notification plugin treats critical urgency as immortal and
/// ignores the sender's timeout, so a 100% toast stays until it is dismissed.
/// [omacom/omarchy#12964](https://github.com/omacom/omarchy/pull/12964)
/// honours a positive timeout instead, capped at 30s, while a critical toast
/// sent without one still stays — hence asking for the cap.
#[cfg(target_os = "linux")]
const CRITICAL_TIMEOUT_MS: &str = "30000";

/// Urgency band for one notification. `notify-send`'s `-u` maps directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgency {
    /// `threshold <= pct < 100`.
    Normal,
    /// The window is exhausted (`pct >= 100`).
    Critical,
}

#[cfg(target_os = "linux")]
impl Urgency {
    fn as_arg(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Critical => "critical",
        }
    }
}

/// One notification to deliver. `key` is the dedupe identity the state map
/// is keyed by; title/body are already sanitized for a subprocess boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub urgency: Urgency,
    pub key: String,
}

/// One metric row of a refresh, as the shared panel projection shapes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricRow {
    /// Window label ("Session (5h)", "Weekly (7d)"). Stable per vendor, so it
    /// is the window half of the dedupe key.
    pub window: String,
    pub percent: u16,
    /// Vendor-reported absolute reset instant. `None` when the vendor does
    /// not state one — then the body carries no ETA, by rule.
    pub reset_at: Option<DateTime<Utc>>,
}

/// The notification-relevant projection of one vendor entry's refresh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshInput {
    /// The report entry id (`vendor` or `vendor@account`) — the entry half of
    /// the dedupe key.
    pub entry_id: String,
    /// Canonical provider display name (`VendorId::display_name`).
    pub vendor: String,
    pub account: Option<String>,
    pub rows: Vec<MetricRow>,
}

/// Decide which notifications one refresh raises.
///
/// `state` is mutated to the post-decision map; the caller persists it (see
/// [`run_at`]) so re-arms survive restarts. Rules:
///
/// - Fire when `percent >= threshold` and the key is not marked notified.
///   Band: `>= 100` is [`Urgency::Critical`], otherwise [`Urgency::Normal`].
/// - A key re-arms when usage drops below `threshold - 7`, or when the
///   window's `reset_at` moves to a later instant (a new window). The same
///   crossing therefore never fires twice.
pub fn decide(
    input: &RefreshInput,
    threshold: u8,
    state: &mut NotifyState,
    now: DateTime<Utc>,
) -> Vec<Notification> {
    let mut fired = Vec::new();
    let threshold = threshold as i32;
    let rearm_below = threshold - HYSTERESIS_PCT;
    for row in &input.rows {
        let key = format!("{}::{}", input.entry_id, row.window);
        let pct = row.percent as i32;
        if pct >= threshold {
            let armed = match state.entries.get(&key) {
                None => true,
                Some(record) => reset_moved_later(row.reset_at, record.reset_at_snapshot),
            };
            if armed {
                fired.push(threshold_notification(input, row, now));
                state.entries.insert(
                    key,
                    KeyRecord {
                        notified_at: now,
                        reset_at_snapshot: row.reset_at,
                    },
                );
            }
        } else if pct < rearm_below {
            // Drop below the hysteresis band: forget the crossing so the next
            // one can fire. Persisted by the caller like any other change.
            state.entries.remove(&key);
        }
    }
    fired
}

/// `true` when `current` is a later reset than `snapshot` — a `None → Some`
/// change counts (the vendor started reporting a reset), `Some → None` and
/// backwards moves do not, and so do not moves within
/// [`RESET_MOVE_TOLERANCE_SECS`], which are the same window reported again.
fn reset_moved_later(current: Option<DateTime<Utc>>, snapshot: Option<DateTime<Utc>>) -> bool {
    match (current, snapshot) {
        (Some(current), Some(snapshot)) => {
            current.signed_duration_since(snapshot).num_seconds() > RESET_MOVE_TOLERANCE_SECS
        }
        (Some(_), None) => true,
        _ => false,
    }
}

fn threshold_notification(
    input: &RefreshInput,
    row: &MetricRow,
    now: DateTime<Utc>,
) -> Notification {
    let who = match &input.account {
        Some(account) => format!("{} · {}", input.vendor, account),
        None => input.vendor.clone(),
    };
    let title = sanitize_untrusted_line(&format!("{} — {} at {}%", who, row.window, row.percent));
    let mut body = format!("{}% of the {} used", row.percent, row.window);
    if let Some(reset_at) = row.reset_at {
        body.push_str(&format!(
            " · resets {} ({})",
            countdown::format(Some(reset_at), now),
            format::local_time_hm(reset_at)
        ));
    }
    Notification {
        title,
        body: sanitize_untrusted_line(&body),
        urgency: if row.percent >= 100 {
            Urgency::Critical
        } else {
            Urgency::Normal
        },
        key: format!("{}::{}", input.entry_id, row.window),
    }
}

// ─── Persisted dedupe state ────────────────────────────────────────────────

/// Map of dedupe key → when it was notified and the reset instant that was
/// current at the time. `notified_at` is bookkeeping (inspectable state);
/// re-arming reads `reset_at_snapshot`. On disk this is the bare map, not a
/// wrapper object.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NotifyState {
    entries: BTreeMap<String, KeyRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct KeyRecord {
    notified_at: DateTime<Utc>,
    reset_at_snapshot: Option<DateTime<Utc>>,
}

impl NotifyState {
    #[cfg(test)]
    fn is_notified(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Read the state file. Missing or corrupt counts as nothing notified —
    /// a corrupt file must never pin a vendor silent or double-fire forever.
    fn read_from(path: &Path) -> Self {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    /// Atomic write (tempfile + persist) with mode 0600, matching the cache
    /// sidecar discipline.
    fn write_to(&self, path: &Path) -> crate::error::Result<()> {
        let bytes = serde_json::to_vec(&self.entries)?;
        crate::cache::atomic_write(path, &bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(path, perms);
            }
        }
        Ok(())
    }
}

// ─── Delivery seam ─────────────────────────────────────────────────────────

/// Delivery backend. Implementations own one platform each.
///
/// `deliver` returns nothing and must never panic: a failing or missing
/// notifier is ALWAYS a silent no-op, and nothing in this module may change
/// a caller's return value or exit code.
pub trait NotifySink {
    fn deliver(&mut self, notification: &Notification);
}

/// The do-nothing sink for platforms without a desktop delivery backend.
#[cfg(any(not(target_os = "linux"), test))]
#[derive(Debug, Default)]
pub struct NoopSink;

#[cfg(any(not(target_os = "linux"), test))]
impl NotifySink for NoopSink {
    fn deliver(&mut self, _notification: &Notification) {}
}

/// Linux `notify-send` sink. argv only — never a shell — with stderr nulled:
/// the notification daemon's complaints are not the bar's problem. A missing
/// binary or a failed spawn is swallowed; a hanging one is killed after
/// [`SPAWN_KILL_AFTER`].
#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct NotifySendSink {
    program: String,
}

#[cfg(target_os = "linux")]
impl Default for NotifySendSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
impl NotifySendSink {
    pub fn new() -> Self {
        Self {
            program: "notify-send".to_string(),
        }
    }

    /// Point at a specific executable — the seam the failing-notifier guard
    /// test uses, and the way a future desktop integration pins a path.
    pub fn with_program(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
        }
    }

    /// `notify-send` arguments for one notification, program name aside.
    ///
    /// The title and body come last: `notify-send` reads the trailing
    /// positionals as those two, so every switch has to precede them.
    fn argv(&self, notification: &Notification) -> Vec<String> {
        let mut args = vec![
            "-a".to_string(),
            "ai-usagebar".to_string(),
            "-c".to_string(),
            "quota".to_string(),
            "-u".to_string(),
            notification.urgency.as_arg().to_string(),
        ];
        if notification.urgency == Urgency::Critical {
            args.push("-t".to_string());
            args.push(CRITICAL_TIMEOUT_MS.to_string());
        }
        args.push(notification.title.clone());
        args.push(notification.body.clone());
        args
    }
}

#[cfg(target_os = "linux")]
impl NotifySink for NotifySendSink {
    fn deliver(&mut self, notification: &Notification) {
        let spawned = std::process::Command::new(&self.program)
            .args(self.argv(notification))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        if let Ok(child) = spawned {
            reap(child);
        }
    }
}

/// Wait for the notifier, killing it at the cap. Desktop delivery normally
/// returns in milliseconds; the cap only matters when the service is wedged.
#[cfg(target_os = "linux")]
fn reap(mut child: std::process::Child) {
    let deadline = std::time::Instant::now() + SPAWN_KILL_AFTER;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(_) => return,
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn production_sink() -> Box<dyn NotifySink + Send> {
    #[cfg(target_os = "linux")]
    return Box::new(NotifySendSink::new());
    #[cfg(not(target_os = "linux"))]
    return Box::new(NoopSink);
}

// ─── Orchestration ─────────────────────────────────────────────────────────

/// Production entry point: the real state path and platform sink. Returns
/// nothing, always — the caller wraps it in `let _ =`.
pub async fn run(input: RefreshInput, threshold: u8) {
    let Some(state_path) = default_state_path() else {
        return;
    };
    // The lock wait and the bounded notifier spawn are blocking; the binary runs
    // a current-thread runtime, so neither belongs on the reactor thread.
    let joined = tokio::task::spawn_blocking(move || {
        let mut sink = production_sink();
        run_at(
            &input,
            threshold,
            &state_path,
            LOCK_TIMEOUT,
            sink.as_mut(),
            Utc::now(),
        );
    });
    let _ = joined.await;
}

/// `~/.cache/ai-usagebar/notifications.json` (or under `$XDG_CACHE_HOME`).
/// Tests never call this — they pass their own path to [`run_at`].
pub fn default_state_path() -> Option<PathBuf> {
    crate::cache::xdg_cache_dir()
        .ok()
        .map(|base| base.join("ai-usagebar").join("notifications.json"))
}

/// The whole check, serialized against any concurrent writer by the same
/// flock pattern the vendor caches use. On lock timeout the check is skipped
/// entirely — a missed notification beats a double one. State is persisted
/// BEFORE delivery: a write failure then costs a missed notification (the
/// preferred failure) rather than a repeat.
pub fn run_at(
    input: &RefreshInput,
    threshold: u8,
    state_path: &Path,
    lock_timeout: Duration,
    sink: &mut dyn NotifySink,
    now: DateTime<Utc>,
) {
    let lock_path = state_path.with_file_name(".notifications.lock");
    let Ok(_guard) = crate::cache::acquire_lock(&lock_path, lock_timeout) else {
        return;
    };
    let mut state = NotifyState::read_from(state_path);
    let before = serde_json::to_string(&state.entries).unwrap_or_default();
    let fired = decide(input, threshold, &mut state, now);
    if serde_json::to_string(&state.entries).unwrap_or_default() != before
        && state.write_to(state_path).is_err()
    {
        // Could not persist the marks: delivering anyway would repeat the
        // notification on the next fresh fetch. Skip instead.
        return;
    }
    for notification in &fired {
        sink.deliver(notification);
    }
}

// ─── Shared-assembly projection ────────────────────────────────────────────

impl RefreshInput {
    /// Project one entry's completed state into the notification input. Rows
    /// come from the same projection the report reads, so a notification names
    /// exactly the window the panel shows.
    pub(crate) fn from_tab(
        tab: &crate::provider::TabId,
        state: &crate::provider::TabState,
        now: DateTime<Utc>,
    ) -> Option<Self> {
        use crate::provider::TabState;
        use crate::sections::project;

        let TabState::Ready(_) = state else {
            return None;
        };
        let vendor = tab.vendor.display_name().to_string();
        let entry_id = crate::report::tab_id(tab);
        // The tolerance only shapes pacing footnotes, which the notification
        // body never reads; any value behaves identically here.
        let rows = metric_rows(project(state, now, 0));
        Some(Self {
            entry_id,
            vendor,
            account: tab.account.clone(),
            rows,
        })
    }
}

/// The metric rows one refresh checks, each named by the window it measures.
///
/// A plain-text row with no value is a heading: the metrics below it take its
/// name into their own ("Weekly · Gemini") until any other row except a
/// spacer. A dedupe key built from a bare label would let a heading's row and
/// a namesake under it clear each other's record, and the same notification
/// would fire again on every refresh.
fn metric_rows(
    sections: impl IntoIterator<Item = crate::sections::SectionProjection>,
) -> Vec<MetricRow> {
    use crate::sections::Section;

    // A heading is a text row with no value; it covers the metrics below it
    // until any other row except a spacer.
    let mut heading: Option<String> = None;
    let mut rows = Vec::new();
    for projected in sections {
        match projected.section {
            Section::Text { label, value } if value.is_empty() => heading = Some(label),
            Section::Metric { label, pct, .. } => {
                let window = match &heading {
                    Some(heading) => format!("{heading} · {label}"),
                    None => label,
                };
                rows.push(MetricRow {
                    window,
                    percent: pct,
                    reset_at: projected.reset_at,
                });
            }
            Section::Spacer => {}
            _ => heading = None,
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    fn at(day: u32, hour: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, day, hour, min, 0).unwrap()
    }

    fn row(window: &str, percent: u16, reset_at: Option<DateTime<Utc>>) -> MetricRow {
        MetricRow {
            window: window.to_string(),
            percent,
            reset_at,
        }
    }

    fn input(rows: Vec<MetricRow>) -> RefreshInput {
        RefreshInput {
            entry_id: "openrouter@work".into(),
            vendor: "OpenRouter".into(),
            account: Some("work".into()),
            rows,
        }
    }

    /// Sink that records deliveries instead of spawning anything.
    #[derive(Default)]
    struct RecorderSink {
        delivered: Vec<Notification>,
    }

    impl NotifySink for RecorderSink {
        fn deliver(&mut self, notification: &Notification) {
            self.delivered.push(notification.clone());
        }
    }

    fn state_path() -> (TempDir, PathBuf) {
        crate::cache::closed_temp_file("notifications.json", None)
    }

    // — Decision: threshold crossing ————————————————

    #[test]
    fn crossing_at_exactly_the_threshold_fires_normal() {
        let mut state = NotifyState::default();
        let now = at(23, 12, 0);
        let reset = at(24, 12, 0);
        let fired = decide(
            &input(vec![row("Session (5h)", 97, Some(reset))]),
            97,
            &mut state,
            now,
        );
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].urgency, Urgency::Normal);
        assert_eq!(fired[0].key, "openrouter@work::Session (5h)");
        assert_eq!(fired[0].title, "OpenRouter · work — Session (5h) at 97%");
        // Sep 23 12:00 → Sep 24 12:00 is exactly one day: "1d 0h", with the
        // reset's local clock time beside it (rendered by the shared helper,
        // so the assertion holds on any machine timezone).
        assert_eq!(
            fired[0].body,
            format!(
                "97% of the Session (5h) used · resets 1d 0h ({})",
                format::local_time_hm(reset)
            )
        );
    }

    #[test]
    fn below_the_threshold_fires_nothing() {
        let mut state = NotifyState::default();
        let fired = decide(
            &input(vec![row("Session (5h)", 96, None)]),
            97,
            &mut state,
            at(23, 12, 0),
        );
        assert!(fired.is_empty());
        assert!(state.entries.is_empty());
    }

    #[test]
    fn exhausted_windows_fire_critical() {
        let mut state = NotifyState::default();
        let fired = decide(
            &input(vec![row("Weekly (7d)", 100, None)]),
            97,
            &mut state,
            at(23, 12, 0),
        );
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].urgency, Urgency::Critical);
        assert_eq!(fired[0].title, "OpenRouter · work — Weekly (7d) at 100%");
    }

    #[test]
    fn the_same_crossing_never_fires_twice() {
        let mut state = NotifyState::default();
        let high = input(vec![row("Weekly (7d)", 98, Some(at(30, 0, 0)))]);
        let now = at(23, 12, 0);
        assert_eq!(decide(&high, 97, &mut state, now).len(), 1);
        assert_eq!(decide(&high, 97, &mut state, now).len(), 0);
        assert_eq!(decide(&high, 97, &mut state, now).len(), 0);
    }

    // — Decision: hysteresis re-arm ——————————————————

    #[test]
    fn hysteresis_rearms_only_below_threshold_minus_seven() {
        let mut state = NotifyState::default();
        let now = at(23, 12, 0);
        let fires_at = |pct: u16, state: &mut NotifyState| {
            let case = input(vec![row("Weekly (7d)", pct, None)]);
            decide(&case, 97, state, now).len()
        };

        assert_eq!(fires_at(97, &mut state), 1, "first crossing fires");
        assert_eq!(fires_at(96, &mut state), 0, "hovering does not re-fire");
        // 90 == threshold - 7: still inside the band, still armed-out.
        assert_eq!(fires_at(90, &mut state), 0);
        assert!(state.is_notified("openrouter@work::Weekly (7d)"));
        // Below the band: the mark is dropped…
        assert_eq!(fires_at(89, &mut state), 0);
        assert!(!state.is_notified("openrouter@work::Weekly (7d)"));
        // …so the next crossing is a new crossing.
        assert_eq!(fires_at(97, &mut state), 1, "re-armed crossing fires");
    }

    #[test]
    fn a_jittered_reset_instant_is_the_same_window() {
        // Observed live on a five-hour window: two fetches four
        // minutes apart reported the same window's reset 0.70s apart, and the
        // strict comparison re-armed the key and re-notified on refresh with
        // nothing changed.
        let mut state = NotifyState::default();
        let now = at(23, 12, 0);
        let reset = at(23, 17, 0);
        let first = input(vec![row("Session (5h)", 100, Some(reset))]);
        assert_eq!(decide(&first, 97, &mut state, now).len(), 1);

        for drift in [
            chrono::Duration::milliseconds(488),
            chrono::Duration::milliseconds(700),
            chrono::Duration::seconds(59),
        ] {
            let jittered = input(vec![row("Session (5h)", 100, Some(reset + drift))]);
            assert_eq!(
                decide(&jittered, 97, &mut state, now).len(),
                0,
                "{drift:?} later is the same window"
            );
        }

        // A window that really rolled over moves the instant by its length.
        let next_window = input(vec![row(
            "Session (5h)",
            100,
            Some(reset + chrono::Duration::hours(5)),
        )]);
        assert_eq!(decide(&next_window, 97, &mut state, now).len(), 1);
    }

    #[test]
    fn a_later_reset_rearms_the_key() {
        let mut state = NotifyState::default();
        let now = at(23, 12, 0);
        let first = input(vec![row("Session (5h)", 98, Some(at(23, 17, 0)))]);
        assert_eq!(decide(&first, 97, &mut state, now).len(), 1);

        // Same reset, still above the threshold: no re-fire.
        assert_eq!(decide(&first, 97, &mut state, now).len(), 0);
        // Earlier or absent resets are not new windows.
        let earlier = input(vec![row("Session (5h)", 98, Some(at(23, 16, 0)))]);
        assert_eq!(decide(&earlier, 97, &mut state, now).len(), 0);
        let absent = input(vec![row("Session (5h)", 98, None)]);
        assert_eq!(decide(&absent, 97, &mut state, now).len(), 0);
        // A later reset is a new window: fires again.
        let next_window = input(vec![row("Session (5h)", 98, Some(at(24, 2, 0)))]);
        assert_eq!(decide(&next_window, 97, &mut state, now).len(), 1);
    }

    #[test]
    fn a_reset_appearing_where_none_was_reported_rearms() {
        let mut state = NotifyState::default();
        let now = at(23, 12, 0);
        let without = input(vec![row("Weekly quota", 99, None)]);
        assert_eq!(decide(&without, 97, &mut state, now).len(), 1);
        assert_eq!(decide(&without, 97, &mut state, now).len(), 0);
        let with = input(vec![row("Weekly quota", 99, Some(at(30, 0, 0)))]);
        assert_eq!(
            decide(&with, 97, &mut state, now).len(),
            1,
            "the vendor starting to report a reset is a new window"
        );
    }

    // — Decision: copy ——————————————————————————————

    #[test]
    fn absent_reset_at_omits_the_eta_from_the_body() {
        let mut state = NotifyState::default();
        let fired = decide(
            &input(vec![row("Weekly (7d)", 97, None)]),
            97,
            &mut state,
            at(23, 12, 0),
        );
        assert_eq!(fired[0].body, "97% of the Weekly (7d) used");
        assert!(!fired[0].body.contains("resets"));
    }

    #[test]
    fn account_label_is_omitted_from_the_title_when_absent() {
        let mut state = NotifyState::default();
        let mut plain = input(vec![row("Weekly (7d)", 97, None)]);
        plain.entry_id = "zai".into();
        plain.vendor = "OpenRouter".into();
        plain.account = None;
        let fired = decide(&plain, 97, &mut state, at(23, 12, 0));
        assert_eq!(fired[0].title, "OpenRouter — Weekly (7d) at 97%");
    }

    #[test]
    fn untrusted_labels_are_sanitized_at_the_subprocess_boundary() {
        let mut state = NotifyState::default();
        let mut hostile = input(vec![row("W\u{1b}[31m", 97, None)]);
        hostile.account = Some("gmail\n\u{202E}evil".into());
        let fired = decide(&hostile, 97, &mut state, at(23, 12, 0));
        assert!(
            !fired[0].title.chars().any(|c| c.is_control()),
            "{}",
            fired[0].title
        );
        assert!(!fired[0].title.contains('\u{202E}'), "{}", fired[0].title);
        assert!(!fired[0].body.contains('\u{1B}'), "{}", fired[0].body);
    }

    // — State persistence ———————————————————————————

    #[test]
    fn state_round_trips_through_disk_and_tolerates_corruption() {
        let (dir, path) = state_path();
        let mut state = NotifyState::default();
        state.entries.insert(
            "openrouter::Weekly (7d)".into(),
            KeyRecord {
                notified_at: at(23, 12, 0),
                reset_at_snapshot: Some(at(30, 0, 0)),
            },
        );
        state.write_to(&path).unwrap();
        assert_eq!(NotifyState::read_from(&path), state);

        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(NotifyState::read_from(&path), NotifyState::default());
        drop(dir);
    }

    #[test]
    #[cfg(unix)]
    fn state_file_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, path) = state_path();
        NotifyState::default().write_to(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        drop(dir);
    }

    #[test]
    fn rearm_survives_a_simulated_restart() {
        let (dir, path) = state_path();
        let now = at(23, 12, 0);
        let mut state = NotifyState::read_from(&path);

        let high = input(vec![row("Weekly (7d)", 97, None)]);
        let low = input(vec![row("Weekly (7d)", 89, None)]);
        assert_eq!(decide(&high, 97, &mut state, now).len(), 1);
        state.write_to(&path).unwrap();

        // New process: same crossing persists.
        let mut state = NotifyState::read_from(&path);
        assert_eq!(decide(&high, 97, &mut state, now).len(), 0);

        // New process: usage drops below the band, mark cleared and written.
        decide(&low, 97, &mut state, now);
        state.write_to(&path).unwrap();

        // New process: the crossing fires again.
        let mut state = NotifyState::read_from(&path);
        assert_eq!(decide(&high, 97, &mut state, now).len(), 1);
        drop(dir);
    }

    // — Orchestration ————————————————————————————————

    #[test]
    fn run_at_persists_marks_and_delivers_through_the_sink() {
        let (dir, path) = state_path();
        let mut sink = RecorderSink::default();
        let input = input(vec![row("Weekly (7d)", 97, None)]);
        run_at(
            &input,
            97,
            &path,
            Duration::from_secs(2),
            &mut sink,
            at(23, 12, 0),
        );
        assert_eq!(sink.delivered.len(), 1);

        // Second run with a fresh sink: marked, nothing delivered.
        let mut second = RecorderSink::default();
        run_at(
            &input,
            97,
            &path,
            Duration::from_secs(2),
            &mut second,
            at(23, 12, 0),
        );
        assert!(second.delivered.is_empty());
        assert!(path.exists());
        drop(dir);
    }

    #[test]
    fn lock_contention_skips_notifying_entirely() {
        let (dir, path) = state_path();
        let lock_path = path.with_file_name(".notifications.lock");
        let _held = crate::cache::acquire_lock(&lock_path, Duration::from_secs(5)).unwrap();

        let mut sink = RecorderSink::default();
        let input = input(vec![row("Weekly (7d)", 100, None)]);
        run_at(
            &input,
            97,
            &path,
            Duration::from_millis(100),
            &mut sink,
            at(23, 12, 0),
        );
        assert!(
            sink.delivered.is_empty(),
            "a missed notification beats a double one"
        );
        assert!(!path.exists(), "no state without the lock");
        drop(dir);
    }

    #[test]
    fn an_unwritable_state_file_skips_delivery_rather_than_repeating() {
        let dir = tempfile::TempDir::new().unwrap();
        // A directory where the state file should be makes the atomic write
        // fail: persisting the mark is a precondition of delivering.
        let state_path = dir.path().join("notifications.json");
        std::fs::create_dir(&state_path).unwrap();
        let mut sink = RecorderSink::default();
        let input = input(vec![row("Weekly (7d)", 97, None)]);
        run_at(
            &input,
            97,
            &state_path,
            Duration::from_secs(2),
            &mut sink,
            at(23, 12, 0),
        );
        assert!(sink.delivered.is_empty());
    }

    /// A 100% toast asks for the 30s Omarchy honours once it stops treating
    /// critical urgency as immortal; a normal one leaves the timeout to the
    /// daemon, which is where Omarchy's own 8s default lives.
    #[test]
    #[cfg(target_os = "linux")]
    fn a_critical_toast_asks_for_a_timeout_and_a_normal_one_does_not() {
        let sink = NotifySendSink::new();
        let toast = |urgency| Notification {
            title: "Claude — Session (5h) at 100%".to_string(),
            body: "resets in 2h".to_string(),
            urgency,
            key: "openrouter::Session (5h)".to_string(),
        };
        let critical_toast = toast(Urgency::Critical);

        let critical = sink.argv(&critical_toast);
        assert!(
            critical
                .windows(2)
                .any(|pair| pair[0] == "-t" && pair[1] == "30000"),
            "a critical toast asks for the cap: {critical:?}"
        );

        let normal = sink.argv(&toast(Urgency::Normal));
        assert!(
            !normal.iter().any(|arg| arg == "-t"),
            "a normal toast leaves the timeout to the daemon: {normal:?}"
        );

        // `notify-send` reads the trailing positionals as title and body, so
        // the switch this test is about must not land after them.
        let tail = &critical[critical.len() - 2..];
        assert_eq!(tail[0], critical_toast.title);
        assert_eq!(tail[1], critical_toast.body);
    }

    /// The guard test for the whole feature: a notifier that cannot even be
    /// spawned must not make the check error, panic, or skip the state
    /// bookkeeping. `run_at` has no failure channel at all — this pins that.
    #[test]
    #[cfg(target_os = "linux")]
    fn a_missing_notifier_binary_is_a_silent_no_op() {
        let (dir, path) = state_path();
        let mut sink = NotifySendSink::with_program("/nonexistent/notify-send");
        let input = input(vec![row("Weekly (7d)", 97, None)]);
        run_at(
            &input,
            97,
            &path,
            Duration::from_secs(2),
            &mut sink,
            at(23, 12, 0),
        );
        assert!(
            path.exists(),
            "state bookkeeping is independent of delivery"
        );
        assert!(
            NotifyState::read_from(&path).is_notified("openrouter@work::Weekly (7d)"),
            "the crossing is marked even though nothing was delivered"
        );
        drop(dir);
    }

    #[test]
    fn noop_sink_records_nothing_and_never_fails() {
        let (dir, path) = state_path();
        let mut sink = NoopSink;
        let input = input(vec![row("Weekly (7d)", 97, None)]);
        run_at(
            &input,
            97,
            &path,
            Duration::from_secs(2),
            &mut sink,
            at(23, 12, 0),
        );
        assert!(path.exists());
        drop(dir);
    }

    #[test]
    fn from_tab_returns_none_for_unready_states() {
        use crate::provider::{TabId, TabState};
        assert!(
            RefreshInput::from_tab(
                &TabId::vendor(crate::vendor::VendorId::Openrouter),
                &TabState::Loading,
                at(23, 12, 0)
            )
            .is_none()
        );
        assert!(
            RefreshInput::from_tab(
                &TabId::vendor(crate::vendor::VendorId::Openrouter),
                &TabState::error("not signed in"),
                at(23, 12, 0)
            )
            .is_none()
        );
    }
}
