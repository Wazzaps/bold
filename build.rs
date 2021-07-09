pub fn main() {
    println!("cargo:rerun-if-changed=src/arch/aarch64/linker.ld");
}
