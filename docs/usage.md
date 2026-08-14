# p4tc-rs Usage Guide

This guide explains how to use the `p4tc` crate to manage P4TC pipelines, tables, and externs from userspace in Rust.

## 1. Setup

**Prerequisites:**
- A Linux environment with a P4TC-enabled kernel.
- `libp4tctrl.so` installed on the system (usually from the `p4tc-ctrl-runt-api` package).

Ensure the pipeline schema JSON files are accessible. You can set the `INTROSPECTION` environment variable to point to the directory containing these files.

## 2. Initialization

Before doing anything, initialize the C library, provision the pipeline, and create a context.

```rust
use p4tc::{Context, Pipeline, Transport};

// 1. Initialize the C library (must be done once)
unsafe { p4tc_sys::p4tc_init() };

// 2. Provision the pipeline into the kernel
let pipe = Pipeline::provision("my_pipeline", None).expect("Failed to provision pipeline");

// 3. Create a communication context
let ctx = Context::new(Transport::Netlink).expect("Failed to create context");
```
The `Pipeline` must be kept alive; it cleans up the pipeline on drop.

## 3. Table Operations

Tables support CRUD operations: Insert, Update, Delete, and Get. Use the `Context` to create builders for these operations.

**Insert:**
```rust
ctx.insert("my_pipeline", "ingress/my_table")
    .key("10.0.0.1")
    .action("ingress/forward")
    .param("eth0")
    .execute()
    .unwrap();
```

**Update:**
Updates are similar to inserts. You specify the key to identify the entry and provide the new action and parameters.
```rust
ctx.update("my_pipeline", "ingress/my_table")
    .key("10.0.0.1")
    .action("ingress/drop")
    .execute()
    .unwrap();
```

**Get:**
Retrieving an entry requires a callback to process the results.
```rust
ctx.get("my_pipeline", "ingress/my_table")
    .key("10.0.0.1")
    .execute(|entries, _phase| {
        for entry in entries {
            println!("Got entry: {:?}", entry);
        }
    })
    .unwrap();
```

**Delete:**
```rust
ctx.delete("my_pipeline", "ingress/my_table")
    .key("10.0.0.1")
    .execute()
    .unwrap();
```

**Dump & Flush:**

`dump` returns a `GetBuilder` without keys (fetches all entries).
`flush` returns a `DeleteBuilder` without keys (deletes all entries).
```rust
// Fetch all entries
ctx.dump("my_pipeline", "ingress/my_table")
    .execute(|entries, _| {
        println!("got {} entries", entries.len());
    })
    .unwrap();

// Delete all entries
ctx.flush("my_pipeline", "ingress/my_table").execute().unwrap();
```

## 4. Extern Operations

Externs (like Registers or Counters) support `update` and `get` operations. They do not support `insert` or `delete` because their instances are statically allocated by the pipeline definition.

**Update Extern:**
```rust
ctx.extern_update("my_pipeline", "Register", "ingress.reg1")
    .key(1)
    .params(&["42", "99"])
    .execute()
    .unwrap();
```

**Get Extern:**

Extern get requires a callback. The `Param.value` field is `Vec<u8>` (raw bytes).
```rust
ctx.extern_get("my_pipeline", "Register", "ingress.reg1")
    .key(1)
    .execute(|entries, _phase| {
        for e in entries {
            println!("key={}", e.key);
            for p in &e.params {
                println!("  {}: {:02x?}", p.name, p.value);
            }
        }
    })
    .unwrap();
```

## 5. Callbacks and Phases

Operations like `get`, `extern_get`, and `subscribe` deliver results via callbacks.
The callback signature is `FnMut(&[TableEntry], Phase)` or `FnMut(&[ExternEntry], Phase)`.

The bindings internally filter phases — your callback is only invoked with actual
data (`Phase::Sot` or `Phase::Mot`). You do not need to match on `Eot` or `Abt`.
Callbacks always return `0` to the C API internally.

## 6. Subscription

Subscribe to real-time table events. Internally, `p4tc_subscribe()` registers
the subscription (returns a `sub_id`), and a background thread runs
`p4tc_subscribe_resp_handle()` where the C library handles events via epoll.

```rust
let mut sub = ctx.subscribe("my_pipeline", "ingress/my_table", |entries, phase| {
    for entry in entries {
        println!("Event phase {:?}: {:?}", phase, entry);
    }
}).unwrap();

// Do other work...

sub.stop();  // calls p4tc_unsubscribe, joins the thread
```

`Subscription` implements `Drop`, so it auto-cleans up when it goes out of
scope. You can also consume it with `sub.join()` (which calls `stop()`
internally).

For filtered subscriptions:

```rust
let mut sub = ctx.subscribe_filtered("pipe", "ingress/t", "srcAddr=10.0.0.1",
    |entries, phase| { /* ... */ }
).unwrap();
```

## 7. Schema Validation

If you enable the `schema` feature in `Cargo.toml`, you can parse the JSON schema to inspect table and extern definitions.

```rust
#[cfg(feature = "schema")]
{
    use p4tc::PipelineSchema;
    let schema = PipelineSchema::load("my_pipeline", None).unwrap();
    if let Some(table) = schema.get_table("ingress/my_table") {
        println!("Table keysize: {}", table.keysize);
    }
}
```

## 8. Error Handling

Operations return a `Result<T, p4tc::Error>`. Common failure modes include:
- `Error::Provision`: Failed to load the pipeline JSON (check `INTROSPECTION`).
- `Error::Crud`: The kernel rejected the operation (e.g., invalid key format or missing entry).
- `Error::Object`: Internal failure when building the FFI object.

## 9. Gotchas

- **Cookie Alignment**: Callbacks use user-provided closures by passing pointers through a thread-local variable, bypassing C's `cookie` parameter safely.
- **ACK Flag**: The `MsgFlags::ACK` flag is handled internally by the library. Do not set it manually.
- **Pipeline Sealing**: The pipeline is sealed and provisioned via `Pipeline::provision`. You cannot perform operations before doing this.
