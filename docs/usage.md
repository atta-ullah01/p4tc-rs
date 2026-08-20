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
Retrieving an entry requires a callback to process the results. When a schema is
loaded (via `INTROSPECTION`), key bytes are automatically decoded into named fields:
```rust
ctx.get("my_pipeline", "ingress/my_table")
    .key("10.0.0.1")
    .execute(|entries, _phase| {
        for entry in entries {
            // entry.key_fields is a HashMap of decoded values
            println!("key: {:?}", entry.key_fields);  // {"dstAddr": Ipv4("10.0.0.1")}
            for act in &entry.actions {
                println!("action: {}", act.name);
                for p in &act.params {
                    println!("  {}: {}", p.name, p.display_value());
                }
            }
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
        for e in entries {
            println!("key={:?}", e.key_fields);
        }
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

Extern get requires a callback. Use `p.decoded()` to get a typed value, or
`p.display_value()` for a human-readable string:
```rust
ctx.extern_get("my_pipeline", "Register", "ingress.reg1")
    .key(1)
    .execute(|entries, _phase| {
        for e in entries {
            println!("key={}", e.key);
            for p in &e.params {
                println!("  {}: {}", p.name, p.display_value());
            }
        }
    })
    .unwrap();
```

## 5. Response Types

### `TableEntry`

| Field | Type | Description |
|---|---|---|
| `table_name` | `String` | Full table path, e.g. `"ingress/nh_table"` |
| `key_fields` | `HashMap<String, DecodedValue>` | Decoded key fields — `{"dstAddr": Ipv4("10.0.0.1")}` |
| `key` | `Vec<u8>` | Raw key bytes (advanced use) |
| `priority` | `u32` | Entry priority |
| `actions` | `Vec<Action>` | List of actions on this entry |

### `Action`

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Full action path, e.g. `"ingress/send_nh"` |
| `params` | `Vec<Param>` | Action parameters |

### `Param`

| Field / Method | Type | Description |
|---|---|---|
| `name` | `String` | Parameter name |
| `decoded()` | `DecodedValue` | Typed value — `Int`, `Ipv4`, `Ipv6`, `Mac`, or `Raw` |
| `display_value()` | `String` | Human-readable string |
| `value` | `Vec<u8>` | Raw bytes (advanced use) |
| `type_name` | `String` | P4 type name from schema |

### `DecodedValue`

Type-aware enum with a `Display` impl:
- `Ipv4(String)` → `"10.0.0.1"`
- `Ipv6(String)` → `"::1"`
- `Mac(String)` → `"00:aa:bb:cc:dd:ee"`
- `Int(u64)` → `2` (ifindex, bit fields)
- `Raw(Vec<u8>)` → hex bytes

## 6. Callbacks and Phases

Operations like `get`, `extern_get`, and `subscribe` deliver results via callbacks.
The callback signature is `FnMut(&[TableEntry], Phase)` or `FnMut(&[ExternEntry], Phase)`.

The bindings internally filter phases — your callback is only invoked with actual
data (`Phase::Sot` or `Phase::Mot`). You do not need to match on `Eot` or `Abt`.

This signature is consistent across all operations: `get`, `dump`, `extern_get`,
and `subscribe` all deliver `(&[T], Phase)`.

## 7. Subscription

Subscribe to real-time table events. Internally, `p4tc_subscribe()` registers
the subscription (returns a `sub_id`), and a background thread runs
`p4tc_subscribe_resp_handle()` where the C library handles events via epoll.

The callback signature matches `get`/`dump` — it receives a slice of `TableEntry`
with decoded `key_fields`:

```rust
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

// Subscription and CRUD need separate contexts.
let ctx_sub = Context::new(Transport::Netlink).unwrap();
let ctx_crud = Context::new(Transport::Netlink).unwrap();

let event_count = Arc::new(AtomicUsize::new(0));
let ec = event_count.clone();
let mut sub = ctx_sub.subscribe("my_pipeline", "ingress/my_table", move |entries, phase| {
    ec.fetch_add(entries.len(), Ordering::Relaxed);
    for e in entries {
        println!("event: key={:?}", e.key_fields);
    }
}).unwrap();

// Trigger events from the CRUD context
ctx_crud.insert("my_pipeline", "ingress/my_table")
    .key("10.0.0.1")
    .action("ingress/drop")
    .execute()
    .unwrap();

std::thread::sleep(std::time::Duration::from_secs(1));

sub.stop();  // calls p4tc_unsubscribe, joins the thread
println!("total events: {}", event_count.load(Ordering::Relaxed));
```

> **Important**: Subscription and CRUD must use **separate** `Context` objects.
> A subscription socket enters a continuous listen state and cannot be used
> for outgoing commands at the same time.

For filtered subscriptions:

```rust
let mut sub = ctx.subscribe_filtered("pipe", "ingress/t", "srcAddr=10.0.0.1",
    |entries, phase| { /* ... */ }
).unwrap();
```

## 8. Schema Validation

If you enable the `schema` feature in `Cargo.toml`, you can parse the JSON schema
at provision time. The schema is used for two things:

1. **Input validation**: Dict-based key and action params are validated against
   the schema and serialized in the correct field order.
2. **Output decoding**: Response key bytes and action params are automatically
   decoded into typed values (`TableEntry.key_fields`, `Param.decoded()`).

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

## 9. Error Handling

Operations return a `Result<T, p4tc::Error>`. Common failure modes include:
- `Error::Provision`: Failed to load the pipeline JSON (check `INTROSPECTION`).
- `Error::Crud`: The kernel rejected the operation (e.g., invalid key format or missing entry).
- `Error::Object`: Internal failure when building the FFI object.

## 10. Notes

1. **Separate Contexts for Subscribe**: A subscription socket is in a continuous
   listen state — always use a dedicated `Context` for subscriptions and a
   separate `Context` for CRUD operations.
2. **ACK Flag**: The `MsgFlags::ACK` flag is handled internally by the library. Do not set it manually.
3. **Pipeline Sealing**: The pipeline is sealed and provisioned via `Pipeline::provision`. You cannot perform operations before doing this.
