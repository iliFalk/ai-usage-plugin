//! Money and percentage formatting shared by the report and the projection.
//!
//! `money`/`usd` decide what money looks like, and `clamp_pct` decides what a
//! percentage on a meter means. Two copies of either had already drifted — a
//! negative balance printed two ways, two gauges disagreeing about "100%" — so
//! every caller goes through here.

use chrono::{DateTime, Local, Utc};

/// A monetary amount, with the sign outside the symbol. `format!("${v:.2}")`
/// puts it inside — `$-5.71` — which reads as a typo rather than as debt, and
/// several providers let a balance go negative: OpenRouter overruns its
/// credits, a provider can report an explicit debt balance.
///
/// The sign is decided from the *rounded* magnitude, so neither a negative
/// zero off the wire nor a sub-cent debt can produce the nonsense `-$0.00`.
///
/// This is the one place that decides what money looks like. Every renderer
/// goes through it so a debt cannot be spelled two ways in two panels.
pub fn money(v: f64, currency: &str) -> String {
    let magnitude = format!("{:.2}", v.abs());
    let sign = if v < 0.0 && magnitude != "0.00" {
        "-"
    } else {
        ""
    };
    with_currency(sign, &magnitude, Some(currency))
}

/// Attach a currency to an already-formatted magnitude and sign.
///
/// The one table that decides which currencies get a symbol. [`money`] works in
/// `f64` at two decimals and [`crate::usage::fmt_minor`] works in integer minor
/// units at the currency's own scale — two different numbers, but they must not
/// disagree about what a euro looks like. They did: `money` rendered EUR as
/// `3.50 EUR` while `fmt_minor` rendered the same currency as `€3.50`.
///
/// `None` means a payload that predates any currency field; those were always
/// USD. A code with no symbol here trails the code instead of guessing one,
/// which is still truthful — rendering R$ 141.57 as "$141.57" is a claim about
/// the wrong currency, the same class of defect as a fabricated number.
pub fn with_currency(sign: &str, number: &str, currency: Option<&str>) -> String {
    match currency {
        None | Some("USD") => format!("{sign}${number}"),
        Some("BRL") => format!("{sign}R${number}"),
        Some("EUR") => format!("{sign}€{number}"),
        Some("GBP") => format!("{sign}£{number}"),
        Some("JPY") | Some("CNY") => format!("{sign}¥{number}"),
        Some(other) => format!("{sign}{number} {other}"),
    }
}

/// [`money`] for the providers that only ever bill in dollars.
pub fn usd(v: f64) -> String {
    money(v, "USD")
}

/// **A shared helper, not a chokepoint.** Every float percentage that is
/// rounded and held to 0–100 routes through here, and an `i32` caller widens
/// the result with `i32::from`. Nothing enforces that. Some percentages follow
/// a different rule on purpose and do not come here: a whole-number percent
/// that a parser already rounded is clamped as an integer before it is cast
/// into `Metric.pct`, and a few vendors round without clamping, or reject an
/// out-of-range value instead of holding it. Prefer this for new callers.
///
/// # Examples
///
/// ```
/// use ai_usagebar::format::clamp_pct;
///
/// assert_eq!(clamp_pct(24.5), 25);
/// assert_eq!(clamp_pct(-3.0), 0);
/// assert_eq!(clamp_pct(140.0), 100);
/// assert_eq!(clamp_pct(f64::NAN), 0);
/// ```
pub fn clamp_pct(v: f64) -> u16 {
    if v.is_nan() {
        0
    } else {
        v.round().clamp(0.0, 100.0) as u16
    }
}

pub fn local_time_hm(when: DateTime<Utc>) -> String {
    when.with_timezone(&Local).format("%H:%M").to_string()
}

pub fn local_time_hms(when: DateTime<Utc>) -> String {
    when.with_timezone(&Local).format("%H:%M:%S").to_string()
}

pub fn local_date_hm(when: DateTime<Utc>) -> String {
    when.with_timezone(&Local)
        .format("%b %-d %H:%M")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_percentage_is_rounded_held_to_the_meter_and_nan_safe() {
        assert_eq!(clamp_pct(0.0), 0);
        assert_eq!(clamp_pct(100.0), 100);
        assert_eq!(clamp_pct(24.5), 25);
        assert_eq!(clamp_pct(24.4), 24);
        // Out of range in either direction stops at the end of the meter
        // rather than drawing past it or wrapping.
        assert_eq!(clamp_pct(-0.4), 0);
        assert_eq!(clamp_pct(-9_999.0), 0);
        assert_eq!(clamp_pct(140.0), 100);
        assert_eq!(clamp_pct(f64::INFINITY), 100);
        assert_eq!(clamp_pct(f64::NEG_INFINITY), 0);
        // A denominator off the wire can be zero, and `0.0 / 0.0` is NaN. That
        // is "nothing to draw", not a panic and not a cast to garbage.
        assert_eq!(clamp_pct(f64::NAN), 0);
    }

    #[test]
    fn usd_keeps_the_sign_outside_the_symbol() {
        assert_eq!(usd(74.5), "$74.50");
        assert_eq!(usd(0.0), "$0.00");
        assert_eq!(usd(-5.71), "-$5.71");
        // A negative zero off the wire is not a debt, and must not print as
        // one — `format!("{:.2}")` alone would render it "$-0.00".
        assert_eq!(usd(-0.0), "$0.00");
        // Neither is a debt too small to show a cent.
        assert_eq!(usd(-0.001), "$0.00");
        // One that does round to a cent keeps its sign.
        assert_eq!(usd(-0.006), "-$0.01");
    }

    #[test]
    fn money_places_the_sign_ahead_of_every_currency() {
        assert_eq!(money(20.0, "CNY"), "¥20.00");
        assert_eq!(money(-20.0, "CNY"), "-¥20.00");
        // A currency trails its code only when this table has no symbol for
        // it; the sign still leads either way.
        assert_eq!(money(3.5, "SEK"), "3.50 SEK");
        assert_eq!(money(-3.5, "SEK"), "-3.50 SEK");
        // usd() is the same policy, not a second one.
        assert_eq!(money(-5.71, "USD"), usd(-5.71));
        assert_eq!(money(-0.0, "CNY"), "¥0.00");
    }

    #[test]
    fn timestamps_are_local_and_fixed_width() {
        let when = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        assert_eq!(local_time_hm(when).len(), 5);
        assert_eq!(local_time_hms(when).len(), 8);
        assert!(!local_date_hm(when).is_empty());
    }
}
