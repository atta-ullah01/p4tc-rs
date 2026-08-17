// Example: event subscription.
// Uses a separate context for CRUD while subscription is active.
//
// Pipeline setup (inside the P4TC VM):
//   tar xzf examples/register.tgz -C ~
//   cd ~/register
//   sudo tc p4template del pipeline/register 2>/dev/null; true
//   sudo tc p4template del extern/root/Register 2>/dev/null; true
//   sudo INTROSPECTION=./generated bash generated/register.template
//
// Build & run (see README.md for build prerequisites):
//   cd /path/to/p4tc-rs
//   cargo build --example subscribe -p p4tc --features schema
//   sudo INTROSPECTION=~/register/generated ./target/debug/examples/subscribe

use p4tc::{Context, Pipeline, Transport};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;

const PIPE: &str = "register";
const TABLE: &str = "ingress/nh_table";

fn main() {
    unsafe { p4tc_sys::p4tc_init() };
    let _pipe = Pipeline::provision(PIPE, None)
        .expect("provision failed (is INTROSPECTION set?)");

    // Subscription and CRUD need separate contexts.
    let ctx_sub = Context::new(Transport::Netlink)
        .expect("context creation failed");
    let ctx_crud = Context::new(Transport::Netlink)
        .expect("context creation failed");

    // Start subscription — callback receives (&[TableEntry], Phase)
    println!("subscribe ...");
    let event_count = Arc::new(AtomicUsize::new(0));
    let ec = event_count.clone();
    let mut sub = ctx_sub.subscribe(PIPE, TABLE, move |entries, phase| {
        ec.fetch_add(entries.len(), Ordering::Relaxed);
        println!("  event: phase={:?}, {} entries", phase, entries.len());
    }).expect("subscribe failed");
    println!("  active={}", sub.active());

    // Trigger some events (on a different context)
    println!("insert (triggers event) ...");
    ctx_crud.insert(PIPE, TABLE)
        .key("10.0.0.1")
        .action("ingress/drop")
        .execute()
        .unwrap();

    println!("delete (triggers event) ...");
    ctx_crud.delete(PIPE, TABLE)
        .key("10.0.0.1")
        .execute()
        .unwrap();

    // Give the background thread time to receive the events
    std::thread::sleep(Duration::from_secs(1));

    // Stop
    println!("stop ...");
    sub.stop();
    println!("  active={}", sub.active());
    println!("  total events: {}", event_count.load(Ordering::Relaxed));

    println!("\ndone.");
}
