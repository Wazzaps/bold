use core::panic::PanicInfo;

#[lang = "eh_personality"]
pub extern "C" fn eh_personality() {}

#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
    // println!("+++ Bold Kernel v{} Panic! +++", env!("CARGO_PKG_VERSION"));
    // if let Some(message) = info.message() {
    //     println!("{}", message);
    // }
    // if let Some(location) = info.location() {
    //     println!("at {}", location);
    // }
    // println!("--- Bold Kernel v{} Panic! ---", env!("CARGO_PKG_VERSION"));
    // poweroff(false);
}
