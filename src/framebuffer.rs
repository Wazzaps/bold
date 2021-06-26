use core::fmt::Debug;

pub trait Framebuffer: Debug {
    fn init(&mut self) -> Result<(), ()>;
    fn draw_example(&mut self, variant: u32);
}
