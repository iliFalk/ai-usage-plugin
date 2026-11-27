//! Stable provider identity: the enum the config, the report, and every
//! frontend share.

use std::time::Duration;

use clap::ValueEnum;

use crate::usage::VendorSnapshot;

/// Outer reqwest client timeout shared by the report entry point. Provider
/// fetchers still apply their own tighter per-request timeouts.
pub const HTTP_CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Upper bound on a provider response body. Every one of these endpoints
/// returns a small JSON document, so this is generous by three orders of
/// magnitude while still bounding the damage from a misbehaving proxy or a
/// hijacked endpoint.
pub const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

/// Follow ordinary provider redirects without forwarding non-standard API-key
/// headers to a different origin. Reqwest strips `Authorization` on sensitive
/// redirects, but providers also use headers such as `x-api-key`, which are
/// not covered by that built-in list.
pub fn same_origin_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 10 {
            return attempt.error("too many redirects");
        }
        let Some(origin) = attempt.previous().first() else {
            return attempt.stop();
        };
        let target = attempt.url();
        if target.scheme() == origin.scheme()
            && target.host_str() == origin.host_str()
            && target.port_or_known_default() == origin.port_or_known_default()
        {
            attempt.follow()
        } else {
            attempt.stop()
        }
    })
}

/// Read a response body with an upper bound.
///
/// `Content-Length` is checked first when present, then the body is read in
/// chunks so a lying or absent length cannot get past the cap either.
pub async fn read_body_capped(
    mut resp: reqwest::Response,
    max: usize,
) -> crate::error::Result<Vec<u8>> {
    let too_big = |n: u64| {
        crate::error::AppError::Schema(format!(
            "response body exceeds the {max}-byte limit ({n} bytes); refusing to buffer it"
        ))
    };
    if let Some(len) = resp.content_length()
        && len > max as u64
    {
        return Err(too_big(len));
    }
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        if chunk.len() > max.saturating_sub(buf.len()) {
            return Err(too_big(buf.len().saturating_add(chunk.len()) as u64));
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

/// Stable enum used in config files and in the report.
#[derive(
    Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum VendorId {
    #[serde(rename = "opencode-go")]
    OpenCodeGo,
    Openrouter,
    Deepseek,
}

impl VendorId {
    pub fn slug(self) -> &'static str {
        match self {
            VendorId::OpenCodeGo => "opencode-go",
            VendorId::Openrouter => "openrouter",
            VendorId::Deepseek => "deepseek",
        }
    }

    /// Canonical human-readable name for shared reports and compact UI labels.
    pub fn display_name(self) -> &'static str {
        match self {
            VendorId::OpenCodeGo => "OpenCode Go",
            VendorId::Openrouter => "OpenRouter",
            VendorId::Deepseek => "DeepSeek",
        }
    }

    /// Glyph for a compact bar chip. The panel takes it from `usage --json`
    /// rather than keeping its own provider-icon table.
    pub const fn bar_icon(self) -> &'static str {
        match self {
            VendorId::Openrouter => "󱙺",
            VendorId::Deepseek => "󰧑",
            VendorId::OpenCodeGo => VendorId::OpenCodeGo.short_name(),
        }
    }

    /// Compact three-letter code for the bar. This is the single source for
    /// the `usage --json` `short_name` field and any frontend that wants a
    /// provider tag.
    pub const fn short_name(self) -> &'static str {
        match self {
            VendorId::OpenCodeGo => "ocg",
            VendorId::Openrouter => "opr",
            VendorId::Deepseek => "dsk",
        }
    }

    /// The `config.toml` table this vendor's settings live in — the `Config`
    /// field name. This is the single source for every writer that edits a
    /// vendor section by name, so a section can't be spelled one way by the
    /// parser and another by a writer.
    pub const fn config_section(self) -> &'static str {
        match self {
            VendorId::OpenCodeGo => "opencode-go",
            VendorId::Openrouter => "openrouter",
            VendorId::Deepseek => "deepseek",
        }
    }

    /// Default environment variable holding this provider's key, or `""` for a
    /// provider that has none. This is only the *default*: every key provider
    /// accepts an `api_key_env` override in config, so a frontend showing the
    /// variable a user must set wants `Config::api_key_env_for`, not this.
    pub const fn api_key_env(self) -> &'static str {
        match self {
            VendorId::OpenCodeGo => "OPENCODE_GO_API_KEY",
            VendorId::Openrouter => "OPENROUTER_API_KEY",
            VendorId::Deepseek => "DEEPSEEK_API_KEY",
        }
    }

    pub const fn all() -> &'static [VendorId] {
        &[
            VendorId::OpenCodeGo,
            VendorId::Openrouter,
            VendorId::Deepseek,
        ]
    }

    /// Look a vendor up by its config/report slug — the validation every
    /// by-name writer (the settings surface's provider on/off toggles) goes
    /// through before touching a config section, so a slug that names no
    /// built-in vendor can never become a `[section]`.
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::all().iter().copied().find(|id| id.slug() == slug)
    }
}

/// What a vendor returns from a successful fetch — the same
/// [`Outcome`](crate::outcome::Outcome) every vendor produces, once its own
/// snapshot type has been widened to [`VendorSnapshot`].
pub type VendorOutcome = crate::outcome::Outcome<VendorSnapshot>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_vendor_has_stable_machine_and_display_names() {
        for vendor in VendorId::all() {
            assert!(!vendor.slug().is_empty());
            assert!(!vendor.display_name().is_empty());
            assert!(!vendor.short_name().is_empty());
            assert!(!vendor.api_key_env().is_empty());
            assert_eq!(VendorId::from_slug(vendor.slug()), Some(*vendor));
        }
        assert_eq!(VendorId::OpenCodeGo.slug(), "opencode-go");
        assert_eq!(VendorId::OpenCodeGo.config_section(), "opencode-go");
        assert_eq!(VendorId::Openrouter.display_name(), "OpenRouter");
        assert_eq!(VendorId::Deepseek.short_name(), "dsk");
    }

    #[test]
    fn slugs_are_unique_and_short_names_are_unique() {
        let slugs: BTreeSet<_> = VendorId::all().iter().map(|v| v.slug()).collect();
        assert_eq!(slugs.len(), VendorId::all().len());
        let shorts: BTreeSet<_> = VendorId::all().iter().map(|v| v.short_name()).collect();
        assert_eq!(shorts.len(), VendorId::all().len());
    }

    #[test]
    fn unknown_slug_has_no_vendor() {
        assert_eq!(VendorId::from_slug("not-a-provider"), None);
    }
}
