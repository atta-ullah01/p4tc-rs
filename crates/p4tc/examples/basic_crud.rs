// Example: basic table CRUD (insert, get, update, delete, dump, flush).
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
//   cargo build --example basic_crud -p p4tc --features schema
//   sudo INTROSPECTION=~/register/generated ./target/debug/examples/basic_crud

use p4tc::{Context, Pipeline, Transport};

const PIPE: &str = "register";
const TABLE: &str = "ingress/nh_table";

fn main() {
    unsafe { p4tc_sys::p4tc_init() };
    let _pipe = Pipeline::provision(PIPE, None)
        .expect("provision failed (is INTROSPECTION set?)");
    let ctx = Context::new(Transport::Netlink)
        .expect("context creation failed");

    // Insert
    println!("insert ...");
    ctx.insert(PIPE, TABLE)
        .key("10.0.0.1")
        .action("ingress/send_nh")
        .param("eth0")
        .param("00:aa:bb:cc:dd:ee")
        .param("00:11:22:33:44:55")
        .execute()
        .unwrap();
    println!("  OK");

    // Get (single entry by key, callback-driven)
    println!("get ...");
    ctx.get(PIPE, TABLE)
        .key("10.0.0.1")
        .execute(|entries, phase| {
            println!("  phase={:?}, {} entries", phase, entries.len());
            for e in entries {
                // e.key_fields is a decoded HashMap, e.g. {"dstAddr": Ipv4("10.0.0.1")}
                println!("  table={}, key={:?}, prio={}", e.table_name, e.key_fields, e.priority);
                for a in &e.actions {
                    println!("    action={}", a.name);
                    for p in &a.params {
                        println!("      {}: {}", p.name, p.display_value());
                    }
                }
            }
        })
        .unwrap();

    // Update (change action to drop)
    println!("update ...");
    ctx.update(PIPE, TABLE)
        .key("10.0.0.1")
        .action("ingress/drop")
        .execute()
        .unwrap();
    println!("  OK");

    // Delete
    println!("delete ...");
    ctx.delete(PIPE, TABLE)
        .key("10.0.0.1")
        .execute()
        .unwrap();
    println!("  OK");

    // Insert two entries, dump all, then flush
    println!("insert two entries ...");
    for ip in &["10.0.0.1", "10.0.0.2"] {
        ctx.insert(PIPE, TABLE)
            .key(ip)
            .action("ingress/send_nh")
            .param("eth0")
            .param("00:aa:bb:cc:dd:ee")
            .param("00:11:22:33:44:55")
            .execute()
            .unwrap();
    }
    println!("  OK");

    println!("dump ...");
    ctx.dump(PIPE, TABLE)
        .execute(|entries, phase| {
            println!("  phase={:?}, {} entries", phase, entries.len());
            for e in entries {
                println!("  key={:?}", e.key_fields);
                for a in &e.actions {
                    for p in &a.params {
                        println!("    {}: {}", p.name, p.display_value());
                    }
                }
            }
        })
        .unwrap();

    println!("flush ...");
    ctx.flush(PIPE, TABLE).execute().unwrap();
    println!("  OK");

    println!("\ndone.");
}
