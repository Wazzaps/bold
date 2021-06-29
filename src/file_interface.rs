use crate::framebuffer::FramebufferCM;

pub type IoResult<T> = Result<T, ()>;

pub trait Read {
    fn read(&self, buf: &mut [u8]) -> IoResult<usize>;

    fn read_exact(&self, mut buf: &mut [u8]) -> IoResult<()> {
        let mut left = buf.len();
        while left > 0 {
            let newly_read = self.read(buf)?;
            if newly_read == 0 {
                // EOF
                return Err(());
            }
            assert!(newly_read <= left);

            buf = &mut buf[newly_read..];
            left -= newly_read;
        }
        Ok(())
    }
}

pub trait Write {
    fn write(&self, buf: &[u8]) -> IoResult<usize>;

    fn flush(&self) -> IoResult<()> {
        Ok(())
    }

    fn write_all(&self, mut buf: &[u8]) -> IoResult<()> {
        let mut left = buf.len();
        while left > 0 {
            let newly_written = self.write(buf)?;
            if newly_written == 0 {
                // EOF
                return Err(());
            }
            buf = &buf[newly_written..];
            left -= newly_written;
        }
        Ok(())
    }
}

pub trait Control {
    fn call(&self, msg: FramebufferCM) -> IoResult<()>;
}

pub struct FileInterface {
    pub read: Option<&'static (dyn Read + Sync)>,
    pub write: Option<&'static (dyn Write + Sync)>,
    pub ctrl: Option<&'static (dyn Control + Sync)>,
}
