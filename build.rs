#![feature(exit_status_error)]

use std::env;
use std::env::current_dir;
use std::process::Command;

pub fn main() {
    println!("cargo:rerun-if-changed=src/arch/aarch64/linker.ld");
    println!("cargo:rerun-if-changed=usermode/example_app/main.c");
    println!("cargo:rerun-if-changed=usermode/example_app/Makefile");

    let out_dir = env::var_os("OUT_DIR").unwrap();

    Command::new("make")
        .env("OUT_DIR", out_dir)
        .current_dir(current_dir().unwrap().join("usermode/example_app"))
        .status()
        .unwrap()
        .exit_ok()
        .expect("Failed to run usermode makefile");
}
