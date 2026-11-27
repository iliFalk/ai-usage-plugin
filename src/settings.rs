//! `ai-usagebar settings show|apply` — the settings bridge the Omarchy panel
//! drives.
//!
//! `show` emits a versioned, non-secret description of the configuration: the
//! primary provider, every provider's on/off switch, which credentials are
//! configured (never their values), the `[notifications]` fields, and OpenCode
//! Go's monthly cycle day. `apply` accepts an additive patch on **stdin** so a
//! key never appears in argv or the process environment, validates every field,
//! and writes through `toml_edit` — the existing config keeps its comments,
//! whitespace and unrelated fields, and a file carrying an inline credential is
//! atomically written and `chmod 600`ed.
//!
//! Only what the patch names is written. An untouched field leaves its section
//! exactly as it was, so an apply that only flips a provider never rewrites a
//! credential.

use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, value};

use crate::config::{
    Config, is_valid_env_var_name, read_config_document, set_bool, set_value, write_config_document,
};
use crate::error::{AppError, Result};
use crate::vendor::VendorId;

/// A provider that authenticates with an inline credential. The order of this
/// table is the order of the credential fields in the panel.
pub struct KeyVendor {
    pub id: VendorId,
    pub label: &'static str,
    pub section: &'static str,
    /// Config field that stores the credential.
    pub config_key: &'static str,
    /// Human-readable name shown in the panel's form.
    pub secret_label: &'static str,
    /// Extra hint after the env var. Empty for none.
    pub note: &'static str,
}

pub const KEY_VENDORS: &[KeyVendor] = &[
    KeyVendor {
        id: VendorId::OpenCodeGo,
        label: "OpenCode Go",
        section: VendorId::OpenCodeGo.config_section(),
        config_key: "api_key",
        secret_label: "API key",
        note: "usage quota",
    },
    KeyVendor {
        id: VendorId::Openrouter,
        label: "OpenRouter",
        section: VendorId::Openrouter.config_section(),
        config_key: "api_key",
        secret_label: "API key",
        note: "",
    },
    KeyVendor {
        id: VendorId::Deepseek,
        label: "DeepSeek",
        section: VendorId::Deepseek.config_section(),
        config_key: "api_key",
        secret_label: "API key",
        note: "",
    },
];

/// Per-field text-input state: the pending value and whether the user touched
/// it. Only a dirty field is written back, so an apply never clobbers a key
/// with the placeholder the form was opened with.
#[derive(Debug, Clone, Default)]
pub struct KeyInput {
    pub buf: String,
    pub dirty: bool,
}

impl KeyInput {
    pub fn from_config(initial: Option<&str>) -> Self {
        Self {
            buf: initial.unwrap_or("").to_string(),
            dirty: false,
        }
    }
}

/// One provider's on/off switch, parallel to `VendorId::all()`. `dirty`
/// follows the same only-write-what-the-user-touched rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderSwitch {
    pub enabled: bool,
    pub dirty: bool,
}

/// The mutable state one `show`/`apply` round works on.
#[derive(Debug, Clone)]
pub struct SettingsState {
    /// Enabled providers only. The primary selector must not offer a value that
    /// cannot actually be used.
    pub primary_choices: Vec<VendorId>,
    pub primary: VendorId,
    /// One input per [`KEY_VENDORS`] entry, same order.
    pub keys: Vec<KeyInput>,
    /// One on/off switch per known provider ([`VendorId::all()`] order).
    pub vendors: Vec<ProviderSwitch>,
    /// `[notifications] enabled` toggle. Written back only once toggled.
    pub notify_enabled: bool,
    pub notify_enabled_dirty: bool,
    /// `[notifications] threshold` (1..=100 at save).
    pub notify_threshold: KeyInput,
    /// OpenCode Go billing-cycle reset day (1-28), when configured.
    pub monthly_cycle_day: Option<u8>,
    pub monthly_cycle_day_dirty: bool,
}

impl SettingsState {
    pub fn from_config(cfg: &Config) -> Self {
        Self::from_config_with(cfg, |name| {
            std::env::var_os(name).is_some_and(|v| !v.is_empty())
        })
    }

    /// [`Self::from_config`] with an injected environment lookup.
    ///
    /// The form offers a provider whose env var is already exported, so a user
    /// need not hand-edit `config.toml` to select it. That is a read of ambient
    /// state, which a test must never depend on: the AUR `check()` runs
    /// `cargo test` during `makepkg`, so a test that branched on the real
    /// environment would fail the install for anyone who exports, say,
    /// `OPENROUTER_API_KEY`. Tests pass their own lookup here.
    pub fn from_config_with(cfg: &Config, env_set: impl Fn(&str) -> bool) -> Self {
        let keys = KEY_VENDORS
            .iter()
            .map(|kv| KeyInput::from_config(cfg.inline_api_key(kv.id)))
            .collect();
        let mut primary_choices = cfg.enabled_vendors();
        // A key provider is opt-in: without an inline `api_key` and with the
        // env var empty, the fetch would fail on the first cycle. A user who
        // already exported the env var has to be able to select the provider
        // here — otherwise they would have to hand-edit `config.toml`, which is
        // the workflow this form exists to avoid. We treat a non-empty env var
        // as a sufficient signal of reachability.
        for kv in KEY_VENDORS {
            if primary_choices.contains(&kv.id) {
                continue;
            }
            let env = cfg.api_key_env_for(kv.id);
            let exported = is_valid_env_var_name(env) && env_set(env);
            if cfg.inline_api_key(kv.id).is_some() || exported {
                primary_choices.push(kv.id);
            }
        }
        // A configured but disabled primary is ineffective. Display the first
        // enabled provider instead, without inventing a persisted primary.
        let primary = cfg
            .ui
            .primary
            .filter(|vendor| primary_choices.contains(vendor) && cfg.is_enabled(*vendor))
            .or_else(|| primary_choices.first().copied())
            .unwrap_or(VendorId::OpenCodeGo);
        Self {
            primary_choices,
            primary,
            keys,
            vendors: VendorId::all()
                .iter()
                .map(|id| ProviderSwitch {
                    enabled: cfg.is_enabled(*id),
                    dirty: false,
                })
                .collect(),
            notify_enabled: cfg.notifications.enabled,
            notify_enabled_dirty: false,
            notify_threshold: KeyInput::from_config(Some(&cfg.notifications.threshold.to_string())),
            monthly_cycle_day: cfg.opencode_go.monthly_cycle_day,
            monthly_cycle_day_dirty: false,
        }
    }
}

fn default_config_path() -> Result<PathBuf> {
    // Save back to the same file Config::load() selected, so a save never
    // shadows an existing config the form did not copy.
    crate::config::resolved_path()
        .ok_or_else(|| AppError::Other("could not resolve config dir".into()))
}

/// Write the state to `path`. Writing a non-empty credential also sets that
/// provider's `enabled = true`, and an explicit off toggled in the same apply
/// wins over that.
pub fn save_to_path(state: &SettingsState, path: &Path) -> Result<()> {
    let mut doc = read_config_document(path)?;

    // The key providers are offered before they are enabled, and selecting one
    // is the explicit opt-in — otherwise a user could pick a provider in the
    // form and the fetch would still fail because `enabled = false`. Every
    // other choice remains enabled-only, so no failed provider is persisted as
    // primary.
    if state.primary_choices.contains(&state.primary) {
        set_string(&mut doc, "ui", "primary", state.primary.slug())?;
        if KEY_VENDORS.iter().any(|kv| kv.id == state.primary) {
            set_bool(&mut doc, state.primary.config_section(), "enabled", true)?;
        }
    }

    for (i, kv) in KEY_VENDORS.iter().enumerate() {
        let Some(input) = state.keys.get(i) else {
            continue;
        };
        update_key(&mut doc, kv, input)?;
    }

    // Provider on/off switches, written after the credential fields so an
    // explicit off wins over the enable-a-pasted-key-implied rule above. Only
    // toggled rows are written, so an untouched save never creates a
    // `[<provider>]` section the config never had.
    for (i, id) in VendorId::all().iter().enumerate() {
        let Some(switch) = state.vendors.get(i) else {
            continue;
        };
        if switch.dirty {
            crate::config::set_vendor_enabled_in_doc(&mut doc, id.slug(), switch.enabled)?;
        }
    }

    // [notifications]: each field is written only when the user touched it,
    // so an untouched save leaves an absent/commented section alone.
    if state.notify_enabled_dirty {
        set_bool(&mut doc, "notifications", "enabled", state.notify_enabled)?;
    }
    if state.notify_threshold.dirty {
        let raw = state.notify_threshold.buf.trim();
        match raw.parse::<u8>() {
            Ok(threshold) if (1..=100).contains(&threshold) => {
                set_value(
                    &mut doc,
                    "notifications",
                    "threshold",
                    Some(toml_edit::Value::from(i64::from(threshold))),
                )?;
            }
            _ => {
                return Err(AppError::Other(format!(
                    "[notifications] threshold must be a whole number between 1 and 100, got {raw:?}"
                )));
            }
        }
    }

    // [opencode-go] monthly_cycle_day: written only when the user touched it.
    if state.monthly_cycle_day_dirty {
        match state.monthly_cycle_day {
            Some(day) if (1..=28).contains(&day) => {
                set_value(
                    &mut doc,
                    VendorId::OpenCodeGo.config_section(),
                    "monthly_cycle_day",
                    Some(toml_edit::Value::from(i64::from(day))),
                )?;
            }
            Some(_) => {
                return Err(AppError::Other(format!(
                    "[opencode-go] monthly_cycle_day must be between 1 and 28, got {:?}",
                    state.monthly_cycle_day
                )));
            }
            None => {
                set_value(
                    &mut doc,
                    VendorId::OpenCodeGo.config_section(),
                    "monthly_cycle_day",
                    None,
                )?;
            }
        }
    }

    write_config_document(path, &doc)
}

/// Apply one credential field to the document. Untouched fields are left
/// alone; a field the user cleared is *removed*, so an inline secret can be
/// deleted from the form rather than lingering in the file.
fn update_key(doc: &mut DocumentMut, vendor: &KeyVendor, input: &KeyInput) -> Result<()> {
    if !input.dirty {
        return Ok(());
    }
    if input.buf.is_empty() {
        if let Some(table) = doc
            .get_mut(vendor.section)
            .and_then(toml_edit::Item::as_table_like_mut)
        {
            table.remove(vendor.config_key);
        }
        return Ok(());
    }
    set_string(doc, vendor.section, vendor.config_key, &input.buf)?;
    set_bool(doc, vendor.section, "enabled", true)
}

/// Set or update a string field in a TOML section, preserving comments and
/// formatting of unaffected nodes.
fn set_string(doc: &mut DocumentMut, section: &str, key: &str, new_value: &str) -> Result<()> {
    let table = doc
        .entry(section)
        .or_insert_with(toml_edit::table)
        .as_table_like_mut()
        .ok_or_else(|| AppError::Other(format!("config.toml: [{section}] is not a table")))?;

    if let Some(item) = table.get_mut(key)
        && let Some(v) = item.as_value_mut()
    {
        *v = toml_edit::Value::from(new_value);
        v.decor_mut().set_prefix(" ");
        return Ok(());
    }
    table.insert(key, value(new_value));
    Ok(())
}

/// Versioned, non-secret description consumed by the panel. Inline key values
/// are deliberately represented only as booleans: a long-lived shell process
/// never needs to receive credentials just to draw a settings form.
#[derive(Debug, Serialize)]
struct SettingsSnapshot {
    schema_version: u8,
    primary: String,
    primary_choices: Vec<PrimaryChoice>,
    keys: Vec<KeyStatus>,
    /// Every known provider's on/off switch — enabled ones and disabled ones,
    /// because turning a provider on is the point.
    vendors: Vec<VendorStatus>,
    /// `[notifications] enabled`: whether a quota window crossing the
    /// threshold raises a desktop notification.
    notify_enabled: bool,
    /// `[notifications] threshold`: the percentage (1..=100) that fires it.
    notify_threshold: u8,
    /// OpenCode Go billing-cycle reset day (1-28), when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    monthly_cycle_day: Option<u8>,
}

#[derive(Debug, Serialize)]
struct PrimaryChoice {
    id: String,
    label: String,
}

#[derive(Debug, Serialize)]
struct KeyStatus {
    id: String,
    label: String,
    environment: String,
    secret_label: String,
    note: String,
    configured: bool,
    inline_configured: bool,
    environment_configured: bool,
}

/// One provider row: its shared display name and whether config.toml enables
/// it.
#[derive(Debug, Serialize)]
struct VendorStatus {
    id: String,
    label: String,
    enabled: bool,
}

/// Additive patch accepted on stdin by `ai-usagebar settings apply`. Missing
/// keys remain byte-for-byte untouched. `clear` explicitly removes an inline
/// key, matching the form's empty-field behavior. `vendors` carries provider
/// on/off toggles; its slugs are validated against the built-in provider list
/// before anything is written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyRequest {
    schema_version: u8,
    primary: Option<String>,
    #[serde(default)]
    keys: BTreeMap<String, KeyMutation>,
    #[serde(default)]
    vendors: BTreeMap<String, bool>,
    /// `[notifications] enabled`. Absent = leave untouched.
    #[serde(default)]
    notify_enabled: Option<bool>,
    /// `[notifications] threshold` (1..=100). Absent = leave untouched.
    #[serde(default)]
    notify_threshold: Option<u8>,
    /// OpenCode Go billing-cycle reset day (1-28). Absent = leave untouched.
    #[serde(default)]
    monthly_cycle_day: Option<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase", deny_unknown_fields)]
enum KeyMutation {
    Set { value: String },
    Clear,
}

const SETTINGS_SCHEMA_VERSION: u8 = 1;
const MAX_SETTINGS_REQUEST_BYTES: u64 = 64 * 1024;
const MAX_API_KEY_BYTES: usize = 16 * 1024;

fn snapshot_from_config_with(
    cfg: &Config,
    environment_configured: impl Fn(&str) -> bool,
) -> SettingsSnapshot {
    let state = SettingsState::from_config_with(cfg, &environment_configured);
    let primary_choices = state
        .primary_choices
        .iter()
        .map(|id| PrimaryChoice {
            id: id.slug().to_string(),
            label: id.display_name().to_string(),
        })
        .collect();
    let keys = KEY_VENDORS
        .iter()
        .map(|vendor| {
            let environment = cfg.api_key_env_for(vendor.id);
            let inline_configured = cfg.inline_api_key(vendor.id).is_some();
            let environment_configured = environment_configured(environment);
            KeyStatus {
                id: vendor.id.slug().to_string(),
                label: vendor.label.to_string(),
                environment: environment.to_string(),
                secret_label: vendor.secret_label.to_string(),
                note: vendor.note.to_string(),
                configured: inline_configured || environment_configured,
                inline_configured,
                environment_configured,
            }
        })
        .collect();
    let vendors = VendorId::all()
        .iter()
        .map(|id| VendorStatus {
            id: id.slug().to_string(),
            label: id.display_name().to_string(),
            enabled: cfg.is_enabled(*id),
        })
        .collect();
    SettingsSnapshot {
        schema_version: SETTINGS_SCHEMA_VERSION,
        primary: state.primary.slug().to_string(),
        primary_choices,
        keys,
        vendors,
        notify_enabled: cfg.notifications.enabled,
        notify_threshold: cfg.notifications.threshold,
        monthly_cycle_day: cfg.opencode_go.monthly_cycle_day,
    }
}

fn settings_snapshot_json(cfg: &Config) -> Result<String> {
    Ok(serde_json::to_string(&snapshot_from_config_with(
        cfg,
        |environment| std::env::var_os(environment).is_some_and(|value| !value.is_empty()),
    ))?)
}

#[cfg(test)]
fn settings_snapshot_json_with(
    cfg: &Config,
    environment_configured: impl Fn(&str) -> bool,
) -> Result<String> {
    Ok(serde_json::to_string(&snapshot_from_config_with(
        cfg,
        environment_configured,
    ))?)
}

fn state_from_apply_request(cfg: &Config, raw: &str) -> Result<SettingsState> {
    let request: ApplyRequest = serde_json::from_str(raw)?;
    if request.schema_version != SETTINGS_SCHEMA_VERSION {
        return Err(AppError::Other(format!(
            "unsupported settings schema version {}",
            request.schema_version
        )));
    }

    let mut state = SettingsState::from_config(cfg);
    if let Some(primary) = request.primary {
        let id = VendorId::from_slug(&primary)
            .ok_or_else(|| AppError::Other(format!("unknown primary provider {primary:?}")))?;
        if !state.primary_choices.contains(&id) {
            return Err(AppError::Other(format!(
                "primary provider {primary:?} is not enabled"
            )));
        }
        state.primary = id;
    }

    for (id, mutation) in request.keys {
        let index = KEY_VENDORS
            .iter()
            .position(|vendor| vendor.id.slug() == id)
            .ok_or_else(|| AppError::Other(format!("unknown credential provider {id:?}")))?;
        let input = &mut state.keys[index];
        match mutation {
            KeyMutation::Set { value } => {
                if value.is_empty() {
                    return Err(AppError::Other(format!(
                        "{} for {id:?} is empty; use the clear action to remove it",
                        KEY_VENDORS[index].secret_label
                    )));
                }
                if value.len() > MAX_API_KEY_BYTES {
                    return Err(AppError::Other(format!(
                        "{} for {id:?} exceeds {MAX_API_KEY_BYTES} bytes",
                        KEY_VENDORS[index].secret_label
                    )));
                }
                if value.chars().any(char::is_control) {
                    return Err(AppError::Other(format!(
                        "{} for {id:?} contains control characters",
                        KEY_VENDORS[index].secret_label
                    )));
                }
                input.buf = value;
            }
            KeyMutation::Clear => input.buf.clear(),
        }
        input.dirty = true;
    }

    // Provider on/off toggles: the slug is the whitelist — anything that names
    // no built-in provider is refused, so a patch can never create or flip an
    // arbitrary config section.
    for (slug, enabled) in request.vendors {
        let index = VendorId::all()
            .iter()
            .position(|id| id.slug() == slug)
            .ok_or_else(|| AppError::Other(format!("unknown provider {slug:?}")))?;
        state.vendors[index] = ProviderSwitch {
            enabled,
            dirty: true,
        };
    }

    // [notifications]: the same two fields the panel writes, validated here so
    // a frontend cannot persist a threshold the product would never accept.
    if let Some(enabled) = request.notify_enabled {
        state.notify_enabled = enabled;
        state.notify_enabled_dirty = true;
    }
    if let Some(threshold) = request.notify_threshold {
        if !(1..=100).contains(&threshold) {
            return Err(AppError::Other(format!(
                "[notifications] threshold must be between 1 and 100, got {threshold}"
            )));
        }
        state.notify_threshold = KeyInput::from_config(Some(&threshold.to_string()));
        state.notify_threshold.dirty = true;
    }

    if let Some(day) = request.monthly_cycle_day {
        if !(1..=28).contains(&day) {
            return Err(AppError::Other(format!(
                "[opencode-go] monthly_cycle_day must be between 1 and 28, got {day}"
            )));
        }
        state.monthly_cycle_day = Some(day);
        state.monthly_cycle_day_dirty = true;
    }

    Ok(state)
}

#[cfg(test)]
fn apply_settings_json_to_path(cfg: &Config, raw: &str, path: &Path) -> Result<()> {
    let state = state_from_apply_request(cfg, raw)?;
    save_to_path(&state, path)
}

fn read_settings_request<R: BufRead>(reader: R) -> Result<String> {
    let mut limited = reader.take(MAX_SETTINGS_REQUEST_BYTES + 1);
    let mut bytes = Vec::new();
    limited.read_until(b'\n', &mut bytes)?;
    if bytes.len() as u64 > MAX_SETTINGS_REQUEST_BYTES {
        return Err(AppError::Other(format!(
            "settings request exceeds {MAX_SETTINGS_REQUEST_BYTES} bytes"
        )));
    }
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
    String::from_utf8(bytes)
        .map_err(|_| AppError::Other("settings request is not valid UTF-8".into()))
}

fn apply_settings_from_stdin() -> Result<()> {
    let raw = read_settings_request(std::io::stdin().lock())?;
    let cfg = Config::load()?;
    let state = state_from_apply_request(&cfg, &raw)?;
    save_to_path(&state, &default_config_path()?)
}

/// Entry point for the `settings` subcommand. `show` never emits a secret;
/// `apply` accepts its patch only over stdin so keys do not appear in argv or
/// the process environment.
pub fn run_cli(action: &crate::cli::SettingsAction) -> i32 {
    let result = match action {
        crate::cli::SettingsAction::Show => Config::load()
            .and_then(|cfg| settings_snapshot_json(&cfg))
            .map(|json| println!("{json}")),
        crate::cli::SettingsAction::Apply => {
            apply_settings_from_stdin().map(|()| println!(r#"{{"ok":true}}"#))
        }
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("settings: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(text: &str) -> Config {
        let cfg: Config = toml::from_str(text).unwrap();
        cfg.validate().unwrap();
        cfg
    }

    fn snapshot(cfg: &Config) -> serde_json::Value {
        serde_json::from_str(&settings_snapshot_json_with(cfg, |_| false).unwrap()).unwrap()
    }

    #[test]
    fn the_snapshot_lists_exactly_the_three_providers() {
        let value = snapshot(&cfg(""));
        let ids: Vec<&str> = value["vendors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|vendor| vendor["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, vec!["opencode-go", "openrouter", "deepseek"]);
        let keys: Vec<&str> = value["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|key| key["id"].as_str().unwrap())
            .collect();
        assert_eq!(keys, vec!["opencode-go", "openrouter", "deepseek"]);
    }

    #[test]
    fn the_snapshot_never_carries_a_secret() {
        let value = snapshot(&cfg("[openrouter]\napi_key = \"sk-or-secret\"\n"));
        let text = value.to_string();
        assert!(!text.contains("sk-or-secret"), "{text}");
        assert_eq!(value["keys"][1]["inline_configured"], true);
        assert_eq!(value["keys"][1]["configured"], true);
    }

    #[test]
    fn an_environment_variable_counts_as_configured() {
        let value = settings_snapshot_json_with(&cfg(""), |name| name == "DEEPSEEK_API_KEY")
            .map(|raw| serde_json::from_str::<serde_json::Value>(&raw).unwrap())
            .unwrap();
        assert_eq!(value["keys"][2]["environment_configured"], true);
        assert_eq!(value["keys"][2]["inline_configured"], false);
        assert_eq!(value["keys"][0]["configured"], false);
    }

    #[test]
    fn the_snapshot_carries_the_notification_settings() {
        let value = snapshot(&cfg("[notifications]\nenabled = true\nthreshold = 95\n"));
        assert_eq!(value["notify_enabled"], true);
        assert_eq!(value["notify_threshold"], 95);
    }

    #[test]
    fn an_unknown_provider_slug_is_refused() {
        let cfg = cfg("");
        let err =
            state_from_apply_request(&cfg, r#"{"schema_version":1,"vendors":{"openruter":true}}"#)
                .unwrap_err();
        assert!(err.to_string().contains("unknown provider"), "{err}");
    }

    #[test]
    fn a_threshold_outside_one_to_hundred_is_refused() {
        let cfg = cfg("");
        for bad in [0u8, 101] {
            let raw = format!(r#"{{"schema_version":1,"notify_threshold":{bad}}}"#);
            assert!(state_from_apply_request(&cfg, &raw).is_err(), "{bad}");
        }
        let raw = r#"{"schema_version":1,"notify_threshold":97}"#.to_string();
        assert!(state_from_apply_request(&cfg, &raw).is_ok());
    }

    #[test]
    fn the_patch_writes_only_what_it_names() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[openrouter]\n# keep me\nenabled = true\n").unwrap();
        let cfg = cfg("");
        apply_settings_json_to_path(&cfg, r#"{"schema_version":1,"notify_threshold":91}"#, &path)
            .unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("# keep me"), "{written}");
        assert!(written.contains("threshold = 91"), "{written}");
        assert!(written.contains("enabled = true"), "{written}");
    }

    #[test]
    fn clearing_a_key_removes_the_field_and_leaves_the_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "[deepseek]\nenabled = true\napi_key = \"sk-secret\"\n",
        )
        .unwrap();
        let cfg = cfg("");
        apply_settings_json_to_path(
            &cfg,
            r#"{"schema_version":1,"keys":{"deepseek":{"action":"clear"}}}"#,
            &path,
        )
        .unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(!written.contains("sk-secret"), "{written}");
        assert!(written.contains("enabled = true"), "{written}");
    }

    /// An apply that changes nothing still re-states the primary, so the file
    /// ends up naming it — but it must not invent any other section or field.
    #[test]
    fn an_apply_that_names_nothing_writes_only_the_primary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[openrouter]\nenabled = true\n").unwrap();
        let cfg = cfg("");
        apply_settings_json_to_path(&cfg, r#"{"schema_version":1}"#, &path).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            written,
            "[openrouter]\nenabled = true\n\n[ui]\nprimary = \"openrouter\"\n"
        );
        assert!(!written.contains("[notifications]"), "{written}");
        assert!(!written.contains("api_key"), "{written}");
    }
}
