# figen

`figen` is a Rust library for generating strongly-typed configuration bindings and registries from a compact declarative schema.

It is designed to simplify configuration management, primarily for embedded development, where parsing and wiring configuration keys by hand becomes repetitive and error-prone.

## Why this project exists

Embedded firmware often needs:

- Static, known-at-compile-time configuration structures
- Reliable parsing from key-value sources (files, NVS, RPC backends, etc.)
- A machine-readable registry that clients can fetch to discover capabilities

`figen` reduces boilerplate by generating the config structs, binders, defaults, and registry metadata from a single source of truth.

## Key features

- Works in both `std` and `no_std` environments
- Declarative config schema via `config_registry!`
- Generated strongly-typed config structs
- Default values and optional fields
- Nested keys and indexed-array key support
- Generated registry metadata with versioning
- Custom property types via `TryFrom<&str>` + `config_binder!`

## Quick example

```rust
use figen::config_registry;

config_registry!(
    name = AppConfig
    version = 1

    str_property("telemetry.endpoint", default = "udp://127.0.0.1:14550", max_len = 64)
    num_property("control.pid.kp", default = 10)
    num_property("control.pid.ki", default = 2)
    bool_property("control.enabled", default = true)
);

// Generated:
// - AppConfig (root config struct)
// - APP_CONFIG_REGISTRY (registry static)
```

Loading:

```rust
let loader = MyPropertyLoader::new(); // implement figen::loader::PropertyLoader
let cfg: AppConfig = figen::load_config(&loader)?;
```

## Multiple registries (example)

You can define independent registries in separate modules and avoid naming collisions:

```rust
mod app {
    use figen::config_registry;

    config_registry!(
        name = AppConfig
        version = 1
        num_property("pid.kp", default = 10)
    );
}

mod board {
    use figen::config_registry;

    config_registry!(
        name = BoardConfig
        version = 1
        num_property("gpio.motor_a.pin", default = 12)
    );
}

// app::APP_CONFIG_REGISTRY
// board::BOARD_CONFIG_REGISTRY
```

## Custom types

```rust
use figen::{config_binder, config_registry};

#[derive(Default)]
struct Gain(u16);

impl TryFrom<&str> for Gain {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse::<u16>().map(Gain).map_err(|_| "parse error")
    }
}

config_registry!(
    name = AppConfig
    version = 1
    custom_property("control.gain", default = "42", ty = Gain)
);

config_binder!(Gain);
```

## Feature flags

- `std`: enables `std` support and lazy static registry for runtime-friendly environments
- `serde`: enables serde serialization support for generated registry/config metadata

Common combinations:

- `--no-default-features`
- `--no-default-features --features serde`
- `--no-default-features --features std`
- `--no-default-features --features std --features serde`

## Project status

Early-stage and evolving. Breaking API changes are still possible while the API is being shaped.

## Contributing

Issues and pull requests are welcome. If you propose API changes, include motivation and expected embedded/firmware workflow impact.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
