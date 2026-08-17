// Example: extern update and get.
// Externs only support update and get — no insert or delete.
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
//   cargo build --example extern_ops -p p4tc --features schema
//   sudo INTROSPECTION=~/register/generated ./target/debug/examples/extern_ops

use p4tc::{Context, Pipeline, Transport};

const PIPE: &str = "register";

fn main() {
    unsafe { p4tc_sys::p4tc_init() };
    let _pipe = Pipeline::provision(PIPE, None)
        .expect("provision failed (is INTROSPECTION set?)");
    let ctx = Context::new(Transport::Netlink)
        .expect("context creation failed");

    // Update extern register at index 1
    println!("extern_update ...");
    ctx.extern_update(PIPE, "Register", "ingress.reg1")
        .key(1)
        .params(&["42", "99"])
        .execute()
        .unwrap();
    println!("  OK");

    // Read it back (callback-driven)
    println!("extern_get ...");
    ctx.extern_get(PIPE, "Register", "ingress.reg1")
        .key(1)
        .execute(|entries, phase| {
            println!("  phase={:?}, {} entries", phase, entries.len());
            for e in entries {
                println!("  kind={}, instance={}, key={}", e.kind, e.instance, e.key);
                for p in &e.params {
                    println!("    {}: {}", p.name, p.display_value());
                }
            }
        })
        .unwrap();

    println!("\ndone.");
}
