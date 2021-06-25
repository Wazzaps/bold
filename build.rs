pub fn main() {
    println!("cargo:rerun-if-changed=src/arch/aarch64/build/linker.ld");
    println!("cargo:rerun-if-changed=src/arch/aarch64/build/init.S");
}
