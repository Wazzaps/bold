pub enum FramebufferCM {
    DrawExample {
        variant: u32,
    },
    Clear {
        color: u32,
    },
    DrawChar {
        font: &'static [u8],
        char: u8,
        row: usize,
        col: usize,
    },
}
