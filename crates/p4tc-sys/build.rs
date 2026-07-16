fn main() {
    // Link against libp4tctrl.so at build time.
    println!("cargo:rustc-link-lib=dylib=p4tctrl");

    // Allow overriding the library search path via P4TC_LIB_PATH.
    if let Ok(path) = std::env::var("P4TC_LIB_PATH") {
        println!("cargo:rustc-link-search=native={path}");
    }
}
