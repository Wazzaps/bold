fn main() {
    println!("cargo:rustc-link-lib=static=circle_shim");
    println!("cargo:rustc-link-search=native=/home/david/code/bold/rpi_kern_rs/externals/circle-sys/vendor/sample/08-usbkeyboard");
}
