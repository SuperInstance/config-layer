//! Layered configuration management for Rust applications.
//!
//! # Overview
//!
//! `config-layer` lets you stack multiple configuration sources (defaults, files,
//! environment variables, runtime overrides) into a single merged view. Later
//! layers override earlier ones, just like layered configs in Twelve-Factor apps.
//!
//! ```
//! use config_layer::{ConfigLayer, ConfigValue, LayeredConfig};
//!
//! let mut cfg = LayeredConfig::new();
//!
//! let mut defaults = ConfigLayer::new("defaults");
//! defaults.set("host", ConfigValue::Str("localhost".into()));
//! defaults.set("port", ConfigValue::Int(8080));
//! cfg.add_layer(defaults);
//!
//! let mut env = ConfigLayer::new("env");
//! env.set("port", ConfigValue::Int(3000));
//! cfg.add_layer(env);
//!
//! assert_eq!(cfg.get("host"), Some(&ConfigValue::Str("localhost".into())));
//! assert_eq!(cfg.get("port"), Some(&ConfigValue::Int(3000))); // env wins
//! ```

use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Value type
// ---------------------------------------------------------------------------

/// A configuration value.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

impl fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigValue::Str(s) => write!(f, "{}", s),
            ConfigValue::Int(n) => write!(f, "{}", n),
            ConfigValue::Float(n) => write!(f, "{}", n),
            ConfigValue::Bool(b) => write!(f, "{}", b),
            ConfigValue::Null => write!(f, "null"),
        }
    }
}

impl ConfigValue {
    /// Try to extract a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Try to extract an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ConfigValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to extract a float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(n) => Some(*n),
            ConfigValue::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Try to extract a bool.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Layer
// ---------------------------------------------------------------------------

/// A single configuration layer with a name and key-value pairs.
#[derive(Debug, Clone)]
pub struct ConfigLayer {
    pub name: String,
    values: HashMap<String, ConfigValue>,
}

impl ConfigLayer {
    /// Create a new named layer.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            values: HashMap::new(),
        }
    }

    /// Set a key in this layer.
    pub fn set(&mut self, key: &str, value: ConfigValue) {
        self.values.insert(key.to_string(), value);
    }

    /// Get a key from this layer only.
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        self.values.get(key)
    }

    /// Remove a key.
    pub fn remove(&mut self, key: &str) -> Option<ConfigValue> {
        self.values.remove(key)
    }

    /// All keys in this layer.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Is the layer empty?
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Create a layer from environment variables with a prefix.
    ///
    /// E.g. `from_env_prefix("APP")` will pick up `APP_HOST`, `APP_PORT`, etc.
    pub fn from_env_prefix(name: &str, prefix: &str) -> Self {
        let mut layer = Self::new(name);
        for (key, value) in std::env::vars() {
            if let Some(rest) = key.strip_prefix(prefix) {
                if rest.starts_with('_') {
                    let config_key = rest[1..].to_lowercase();
                    // Try to parse as number or bool, fall back to string
                    let cv = if value == "true" || value == "false" {
                        ConfigValue::Bool(value == "true")
                    } else if let Ok(n) = value.parse::<i64>() {
                        ConfigValue::Int(n)
                    } else if let Ok(n) = value.parse::<f64>() {
                        ConfigValue::Float(n)
                    } else {
                        ConfigValue::Str(value)
                    };
                    layer.set(&config_key, cv);
                }
            }
        }
        layer
    }
}

// ---------------------------------------------------------------------------
// Layered config
// ---------------------------------------------------------------------------

/// Merged, layered configuration.
#[derive(Debug, Clone)]
pub struct LayeredConfig {
    layers: Vec<ConfigLayer>,
}

impl LayeredConfig {
    /// Create an empty config with no layers.
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Number of layers.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Add a layer on top (highest priority).
    pub fn add_layer(&mut self, layer: ConfigLayer) {
        self.layers.push(layer);
    }

    /// Remove the top layer.
    pub fn pop_layer(&mut self) -> Option<ConfigLayer> {
        self.layers.pop()
    }

    /// Insert a layer at a specific position (0 = lowest priority).
    pub fn insert_layer(&mut self, index: usize, layer: ConfigLayer) {
        self.layers.insert(index, layer);
    }

    /// Get the effective value for a key (searches layers top-down).
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        for layer in self.layers.iter().rev() {
            if let Some(v) = layer.get(key) {
                return Some(v);
            }
        }
        None
    }

    /// Get a value with a default.
    pub fn get_or(&self, key: &str, default: ConfigValue) -> ConfigValue {
        self.get(key).cloned().unwrap_or(default)
    }

    /// Set a runtime override (adds a top layer if needed).
    pub fn set(&mut self, key: &str, value: ConfigValue) {
        if let Some(top) = self.layers.last_mut() {
            if top.name == "__runtime" {
                top.set(key, value);
                return;
            }
        }
        let mut rt = ConfigLayer::new("__runtime");
        rt.set(key, value);
        self.add_layer(rt);
    }

    /// Collect all keys across all layers (last-write wins for duplicates).
    pub fn all_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for layer in &self.layers {
            for k in layer.keys() {
                if !keys.contains(k) {
                    keys.push(k.clone());
                }
            }
        }
        keys
    }

    /// Flatten to a single HashMap (resolved values).
    pub fn to_map(&self) -> HashMap<String, ConfigValue> {
        let mut map = HashMap::new();
        for layer in &self.layers {
            for k in layer.keys() {
                if let Some(v) = layer.get(k) {
                    map.insert(k.clone(), v.clone());
                }
            }
        }
        map
    }

    /// Render a debug view showing which layer each key comes from.
    pub fn debug_view(&self) -> String {
        let mut out = String::new();
        for key in &self.all_keys() {
            for layer in self.layers.iter().rev() {
                if let Some(v) = layer.get(key) {
                    out.push_str(&format!(
                        "{} = {}  [{}]\n",
                        key,
                        v,
                        layer.name
                    ));
                    break;
                }
            }
        }
        out
    }
}

impl Default for LayeredConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_override() {
        let mut cfg = LayeredConfig::new();
        let mut defaults = ConfigLayer::new("defaults");
        defaults.set("port", ConfigValue::Int(8080));
        cfg.add_layer(defaults);

        let mut env = ConfigLayer::new("env");
        env.set("port", ConfigValue::Int(3000));
        cfg.add_layer(env);

        assert_eq!(cfg.get("port"), Some(&ConfigValue::Int(3000)));
    }

    #[test]
    fn fallback_to_lower_layer() {
        let mut cfg = LayeredConfig::new();
        let mut defaults = ConfigLayer::new("defaults");
        defaults.set("host", ConfigValue::Str("localhost".into()));
        cfg.add_layer(defaults);

        let mut env = ConfigLayer::new("env");
        env.set("port", ConfigValue::Int(3000));
        cfg.add_layer(env);

        assert_eq!(cfg.get("host"), Some(&ConfigValue::Str("localhost".into())));
    }

    #[test]
    fn runtime_override() {
        let mut cfg = LayeredConfig::new();
        let mut defaults = ConfigLayer::new("defaults");
        defaults.set("debug", ConfigValue::Bool(false));
        cfg.add_layer(defaults);

        cfg.set("debug", ConfigValue::Bool(true));
        assert_eq!(cfg.get("debug"), Some(&ConfigValue::Bool(true)));
    }

    #[test]
    fn get_or_default() {
        let cfg = LayeredConfig::new();
        assert_eq!(
            cfg.get_or("missing", ConfigValue::Str("fallback".into())),
            ConfigValue::Str("fallback".into())
        );
    }

    #[test]
    fn to_map_flattens() {
        let mut cfg = LayeredConfig::new();
        let mut l1 = ConfigLayer::new("base");
        l1.set("a", ConfigValue::Int(1));
        l1.set("b", ConfigValue::Int(2));
        cfg.add_layer(l1);

        let mut l2 = ConfigLayer::new("override");
        l2.set("b", ConfigValue::Int(99));
        cfg.add_layer(l2);

        let map = cfg.to_map();
        assert_eq!(map.get("a"), Some(&ConfigValue::Int(1)));
        assert_eq!(map.get("b"), Some(&ConfigValue::Int(99)));
    }

    #[test]
    fn debug_view_shows_source() {
        let mut cfg = LayeredConfig::new();
        let mut l = ConfigLayer::new("defaults");
        l.set("key", ConfigValue::Str("val".into()));
        cfg.add_layer(l);
        let view = cfg.debug_view();
        assert!(view.contains("[defaults]"));
    }

    #[test]
    fn config_value_conversions() {
        assert_eq!(ConfigValue::Int(42).as_int(), Some(42));
        assert_eq!(ConfigValue::Str("hi".into()).as_str(), Some("hi"));
        assert_eq!(ConfigValue::Bool(true).as_bool(), Some(true));
        assert_eq!(ConfigValue::Float(3.14).as_float(), Some(3.14));
        assert_eq!(ConfigValue::Int(5).as_float(), Some(5.0)); // int → float
    }
}

/// FNV-1a 64 — the digest every substrate in the SuperInstance fleet agrees on.
pub const FNV_OFFSET: u64 = 0xcbf29ce484222325;
pub const FNV_PRIME: u64 = 0x100000001b3;

#[inline]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in bytes {
        h = (h ^ b as u64).wrapping_mul(FNV_PRIME);
    }
    h
}

/// True if this crate's FNV-1a still agrees with the rest of the fleet.
pub fn canary_holds() -> bool {
    fnv1a64("café Δ 日本語".as_bytes()) == 0x024a555471370b18d
}
