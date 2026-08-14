# p4tc-rs

Safe Rust bindings for the P4TC runtime control library.

## Overview

Two-crate workspace wrapping `libp4tctrl` via FFI:

- **p4tc-sys** — raw FFI bindings to `libp4tctrl.so`
- **p4tc** — safe, typed Rust API with builder pattern

## Requirements

- `libp4tctrl.so` installed (from `p4tc-ctrl-runt-api` package)
- P4TC kernel with a provisioned pipeline
- Linux (netlink transport)

## Quick Start

```rust
use p4tc::{Context, Pipeline, Transport};

fn main() {
    unsafe { p4tc_sys::p4tc_init() };
    let _pipe = Pipeline::provision("my_pipeline", None).unwrap();
    let ctx = Context::new(Transport::Netlink).unwrap();

    // Insert
    ctx.insert("my_pipeline", "ingress/my_table")
        .key("10.0.0.1")
        .action("ingress/send")
        .param("eth0")
        .param("00:aa:bb:cc:dd:ee")
        .param("00:11:22:33:44:55")
        .execute()
        .unwrap();

    // Get (callback is required)
    ctx.get("my_pipeline", "ingress/my_table")
        .key("10.0.0.1")
        .execute(|entries, _phase| {
            for entry in entries {
                println!("{:?}", entry);
            }
        })
        .unwrap();

    // Delete
    ctx.delete("my_pipeline", "ingress/my_table")
        .key("10.0.0.1")
        .execute()
        .unwrap();
}
```

See [docs/usage.md](docs/usage.md) for the full API reference covering
update, dump, flush, externs, subscriptions, schema validation, and error handling.

## Building

```bash
cargo build -p p4tc --features schema
```

Set the `INTROSPECTION` environment variable to the directory containing
`<pipeline>.json` schema files if they are not in the current directory.
