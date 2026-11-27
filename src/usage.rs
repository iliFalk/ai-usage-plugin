//! Canonical in-memory representation of "how much have I used my plan".
//!
//! Each provider's snapshot lives in its own variant — they genuinely differ:
//! OpenCode Go reports three quota windows, OpenRouter a credit balance plus
//! key limits and recent models, DeepSeek a plain account balance. Forcing them
//! into a shared shape would either drop information or paper over a real
//! difference. The projection in `sections` is what flattens them for a
//! frontend, so no frontend ever sees a snapshot.

use chrono::{DateTime, Utc};

/// A single usage window — generic enough that every provider with a notion of
/// "% used vs. when does it reset" can express itself with it.
///
/// `utilization_pct` is `0..=100` (integer percent).
/// `resets_at` is `None` when the provider doesn't report a reset time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageWindow {
    pub utilization_pct: i32,
    pub resets_at: Option<DateTime<Utc>>,
    /// Window length (used for pacing math).
    pub window_duration: chrono::Duration,
}

/// DeepSeek — credit balance from `/user/balance`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeepseekSnapshot {
    pub is_available: bool,
    /// Current balance (prefer USD, fallback to CNY).
    pub balance: f64,
    /// Free-granted credits component.
    pub granted: f64,
    /// Topped-up (purchased) credits component.
    pub topped_up: f64,
    /// The currency of the above amounts (currently "USD" or "CNY").
    pub currency: String,
}

impl Eq for DeepseekSnapshot {}

impl Default for DeepseekSnapshot {
    fn default() -> Self {
        Self {
            is_available: false,
            balance: 0.0,
            granted: 0.0,
            topped_up: 0.0,
            currency: String::new(),
        }
    }
}

/// OpenRouter — credit balance + lifetime/daily/weekly/monthly usage from
/// `/api/v1/credits` and `/api/v1/key`.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenRouterSnapshot {
    pub label: String,
    pub total_credits: f64,
    pub total_usage: f64,
    pub usage_daily: f64,
    pub usage_weekly: f64,
    pub usage_monthly: f64,
    pub is_free_tier: bool,
    pub limit: Option<f64>,
    pub limit_remaining: Option<f64>,
    pub recent_models: Vec<String>,
}

impl Eq for OpenRouterSnapshot {}

impl OpenRouterSnapshot {
    /// Spendable credit, **which can be negative**: OpenRouter lets an account
    /// run into debt, and clamping that to zero would report a healthy-looking
    /// `$0.00` to someone who has to top up before anything works again. The
    /// wire fields are each non-negative (see `openrouter::types`), so a
    /// negative result only ever means usage has overrun credits.
    pub fn balance(&self) -> f64 {
        self.total_credits - self.total_usage
    }
    /// Percentage of total_credits consumed (0..=100). Returns 0 when
    /// `total_credits` is 0 (free-tier-only accounts) — there is no
    /// denominator to be a percentage of. Severity does not come from this
    /// number alone: see [`crate::openrouter::vendor::severity`], which treats
    /// a negative [`Self::balance`] as critical regardless of the percentage.
    pub fn consumed_pct(&self) -> i32 {
        if self.total_credits <= 0.0 {
            return 0;
        }
        i32::from(crate::format::clamp_pct(
            (self.total_usage / self.total_credits) * 100.0,
        ))
    }
}

/// One configured provider's snapshot, as the projection consumes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorSnapshot {
    OpenCodeGo(crate::opencode_go::types::Usage),
    Openrouter(OpenRouterSnapshot),
    Deepseek(DeepseekSnapshot),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openrouter_percent_is_zero_without_credits_and_clamped_otherwise() {
        let mut snap = OpenRouterSnapshot {
            label: "default".into(),
            total_credits: 0.0,
            total_usage: 5.0,
            usage_daily: 0.0,
            usage_weekly: 0.0,
            usage_monthly: 0.0,
            is_free_tier: true,
            limit: None,
            limit_remaining: None,
            recent_models: Vec::new(),
        };
        assert_eq!(snap.consumed_pct(), 0);
        snap.total_credits = 10.0;
        assert_eq!(snap.consumed_pct(), 50);
        assert_eq!(snap.balance(), 5.0);
        // Overrun: the balance is negative on purpose, the percentage clamps.
        snap.total_usage = 20.0;
        assert_eq!(snap.consumed_pct(), 100);
        assert_eq!(snap.balance(), -10.0);
    }
}
