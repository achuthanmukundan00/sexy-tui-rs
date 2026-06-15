/// Theme engine — three-layer resolution system.
/// Built-in defaults → TOML config → agent runtime overrides.

pub mod capability;
pub mod config;
pub mod palette;
pub mod tokens;

use std::collections::HashMap;

/// Resolved theme providing render-time styling.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Token values (resolved through all 3 layers).
    values: HashMap<String, String>,
    /// Current capability tier.
    tier: capability::CapabilityTier,
}

impl Theme {
    pub fn new(tier: capability::CapabilityTier) -> Self {
        Theme {
            values: HashMap::new(),
            tier,
        }
    }

    /// Load theme from built-in defaults + TOML config.
    pub fn load(config_path: Option<&str>, tier: capability::CapabilityTier) -> Self {
        let mut theme = Theme::new(tier);
        tokens::apply_defaults(&mut theme.values);
        if let Some(path) = config_path {
            config::load_toml(path, &mut theme.values);
        }
        theme
    }

    /// Apply a foreground color token to text.
    pub fn fg(&self, token: &str, text: &str) -> String {
        let color = self.values.get(token)
            .cloned()
            .unwrap_or_else(|| token.to_string());
        palette::apply_fg(&color, text)
    }

    /// Apply a background color token to text.
    pub fn bg(&self, token: &str, text: &str) -> String {
        let color = self.values.get(token)
            .cloned()
            .unwrap_or_else(|| token.to_string());
        palette::apply_bg(&color, text)
    }

    /// Apply bold styling to text.
    pub fn bold(&self, text: &str) -> String {
        format!("\x1b[1m{}\x1b[22m", text)
    }

    /// Apply dim styling to text.
    pub fn dim(&self, text: &str) -> String {
        format!("\x1b[2m{}\x1b[22m", text)
    }

    /// Resolve an icon token (with capability-aware fallback).
    pub fn icon(&self, token: &str) -> String {
        self.values.get(token)
            .cloned()
            .unwrap_or_else(|| tokens::ascii_fallback(token).to_string())
    }

    /// Get the current capability tier.
    pub fn capability_tier(&self) -> capability::CapabilityTier {
        self.tier
    }

    /// Override a token value at runtime (agent layer — highest priority).
    pub fn override_token(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
    }

    /// Clear a single agent override.
    pub fn clear_override(&mut self, _key: &str) {
        // In the full implementation, this would restore from lower layers
    }

    /// Clear all agent overrides (reload from TOML).
    pub fn clear_all_overrides(&mut self) {
        // In the full implementation, reload from TOML + defaults
    }

    /// List all overrideable token keys.
    pub fn keys(&self) -> Vec<&str> {
        self.values.keys().map(|k| k.as_str()).collect()
    }

    /// Resolve a token value through all layers.
    pub fn resolve<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.values.get(key).and_then(|v| v.parse().ok())
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::new(capability::CapabilityTier::Baseline)
    }
}
