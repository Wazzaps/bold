use alloc::boxed::Box;
use alloc::vec::Vec;
use core::include_bytes;
use spin::Mutex;

pub enum FontState {
    Compressed(&'static [u8]),
    Uncompressed(&'static [u8]),
}

pub struct Font {
    inner: Mutex<FontState>,
}

impl Font {
    pub const fn new(compressed: &'static [u8]) -> Self {
        Self {
            inner: Mutex::new(FontState::Compressed(compressed)),
        }
    }

    pub fn get(&self) -> &'static [u8] {
        let mut inner = self.inner.lock();
        match *inner {
            FontState::Compressed(compressed) => {
                let uncompressed = Font::uncompress(compressed);
                *inner = FontState::Uncompressed(uncompressed);
                uncompressed
            }
            FontState::Uncompressed(uncompressed) => uncompressed,
        }
    }

    fn uncompress(compressed: &[u8]) -> &'static [u8] {
        let mut uncompressed = Vec::new();
        for byte in compressed.iter() {
            for bit in 0..8 {
                if ((*byte >> bit) & 1u8) != 0u8 {
                    uncompressed.extend_from_slice(&[255, 255, 255, 255]);
                } else {
                    uncompressed.extend_from_slice(&[0, 0, 0, 255]);
                }
            }
        }
        uncompressed.leak()
    }
}

pub static ISO88591: Font = Font::new(include_bytes!("iso88591.binfont"));
pub static ISO: Font = Font::new(include_bytes!("iso.binfont"));
pub static TREMOLO: Font = Font::new(include_bytes!("tremolo.binfont"));
pub static VGA: Font = Font::new(include_bytes!("vga.binfont"));
pub static TERMINUS: Font = Font::new(include_bytes!("terminus.binfont"));
