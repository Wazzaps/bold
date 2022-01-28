#![no_std]
extern crate alloc;

type LogHandler = unsafe extern "C" fn(*const u8, usize) -> u32;

extern "C" {
    fn circle_shim_get_class_layout(size: &mut usize, align: &mut usize);
    fn circle_shim_init(this: *mut u8, log_handler: LogHandler) -> u8;
    fn circle_shim_test_keyboard(this: *mut u8);
    fn circle_shim_destroy(this: *mut u8);
}

pub struct CircleShim {
    inner: *mut u8,
}

impl CircleShim {
    pub unsafe fn new(log_handler: LogHandler) -> Self {
        let mut layout_size = 0;
        let mut layout_align = 0;
        circle_shim_get_class_layout(&mut layout_size, &mut layout_align);

        let inner = alloc::alloc::alloc_zeroed(
            alloc::alloc::Layout::from_size_align(layout_size, layout_align)
                .expect("CircleShim: Invalid alloc layout"),
        );

        let res = circle_shim_init(inner, log_handler);
        if res == 0 {
            panic!("CircleShim: Failed to initialize");
        }

        Self { inner }
    }

    pub unsafe fn test_keyboard(&mut self) {
        circle_shim_test_keyboard(self.inner);
    }
}

unsafe impl Send for CircleShim {}

impl Drop for CircleShim {
    fn drop(&mut self) {
        unsafe { circle_shim_destroy(self.inner) };
    }
}
