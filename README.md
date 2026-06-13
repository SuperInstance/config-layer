# Config Layer — Layered Configuration with Priority Override

**Config layering** is a configuration management pattern where multiple named sources — defaults, files, environment variables, runtime overrides — are stacked, with later (higher) layers overriding earlier ones. This crate implements `LayeredConfig`: a stack of `ConfigLayer` objects where `get(key)` searches from the top down and returns the first match.

## Why It Matters

Every well-designed application has layered configuration. The Twelve-Factor App methodology prescribes it: code ships with sensible defaults, deployment overlays environment-specific overrides, and runtime commands override both. Without layering, you end up with `if env_var.exists() { use_it() } else { use_file() } else { use_default() }` scattered everywhere. Config layering makes precedence explicit and uniform: later layers win, period. This is how Kubernetes merges ConfigMaps, how Spring Boot merges `application.yml` profiles, and how 12-factor apps merge defaults → file → env → CLI args.

## How It Works

### Layer Stack

```
Priority (high → low):

  Layer 3: "cli_args"       ← --port=9090
  Layer 2: "env"            ← APP_PORT=8080
  Layer 1: "file"           ← config.json: { "port": 3000 }
  Layer 0: "defaults"       ← port: 5000

  cfg.get("port") → 9090 (from cli_args, highest priority)
  cfg.get("host") → "0.0.0.0" (from file, first match)
```

### Lookup Algorithm

`get(key)` iterates layers from highest to lowest priority:

```
fn get(key):
    for layer in layers.rev():
        if layer has key:
            return layer[key]
    return None
```

**Complexity**: `O(L)` per lookup where `L` = number of layers. For `L ≤ 10` (typical), this is effectively `O(1)`.

### Environment Variable Layer

The `from_env_prefix(name, prefix)` method auto-parses environment variables with a prefix into typed values:

```
APP_HOST=localhost    →  host = "localhost"
APP_PORT=8080         →  port = 8080 (parsed as Int)
APP_DEBUG=true        →  debug = true (parsed as Bool)
```

Parsing tries `bool → i64 → f64 → String` in order.

### Merge Semantics

Layers do **not** merge values — they override entirely. If layer 1 has `{ "server": { "host": "x", "port": 80 } }` and layer 2 has `{ "server": { "port": 9090 } }`, the result for `server` is `{ "port": 9090 }` from layer 2, not a deep-merged `{ "host": "x", "port": 9090 }`. This is override semantics, not merge semantics. Deep merging is a deliberate omission — it's ambiguous and surprising.

## Quick Start

```rust
use config_layer::{ConfigLayer, ConfigValue, LayeredConfig};

let mut cfg = LayeredConfig::new();

// Layer 0: defaults
let mut defaults = ConfigLayer::new("defaults");
defaults.set("host", ConfigValue::Str("localhost".into()));
defaults.set("port", ConfigValue::Int(8080));
cfg.add_layer(defaults);

// Layer 1: environment overrides (higher priority)
let mut env = ConfigLayer::new("env");
env.set("port", ConfigValue::Int(3000));
cfg.add_layer(env);

assert_eq!(cfg.get("host"), Some(&ConfigValue::Str("localhost".into()))); // from defaults
assert_eq!(cfg.get("port"), Some(&ConfigValue::Int(3000)));               // env wins
```

### From Environment Variables

```rust
// Reads APP_HOST, APP_PORT, APP_DEBUG, etc.
let env_layer = ConfigLayer::from_env_prefix("env", "APP");
cfg.add_layer(env_layer);
```

## API

| Type / Method | Description |
|---|---|
| `LayeredConfig` | Stack of layers with merged lookup. |
| `ConfigLayer` | Named layer with key-value pairs. |
| `ConfigValue` | `Str`, `Int`, `Float`, `Bool`, `Null`. Supports `as_str()`, `as_int()`, `as_float()`, `as_bool()`. |
| `cfg.add_layer(layer)` | Push a layer on top (highest priority). |
| `cfg.get(key)` | Top-down search: first match wins. `O(L)`. |
| `cfg.pop_layer()` | Remove and return the top layer. |
| `ConfigLayer::from_env_prefix(name, prefix)` | Auto-parse `PREFIX_*` env vars into typed values. |

## Architecture Notes

Config layering is part of the γ (generation/runtime) side of γ + η = C in SuperInstance. It provides the configuration pipeline that lets the same binary run with different settings across development, staging, and production — defaults are baked in, environment layers override per deployment. See [SuperInstance Architecture](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## References

1. Wiggins, A. (2011). *The Twelve-Factor App*, Factor III: Config. <https://12factor.net/config>
2. Spring Boot Externalized Configuration. <https://docs.spring.io/spring-boot/docs/current/reference/html/features.html#features.external-config>
3. Burns, B. (2018). *Kubernetes: Up and Running* (2nd ed.). O'Reilly. — ConfigMaps and layered config.

## License

MIT
