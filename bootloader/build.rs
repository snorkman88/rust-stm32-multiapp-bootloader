use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    
    // Copy both device.x and memory.x to OUT_DIR
    fs::copy("device.x", out.join("device.x")).unwrap();
    fs::copy("memory.x", out.join("memory.x")).unwrap();
    
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=device.x");
}
