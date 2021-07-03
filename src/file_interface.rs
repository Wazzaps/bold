use crate::framebuffer::FramebufferCM;
use alloc::prelude::v1::Box;
use async_trait::async_trait;

pub type IoResult<T> = Result<T, ()>;

pub trait SyncRead {
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

pub trait SyncWrite {
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

#[async_trait]
pub trait Read {
    async fn read(&self, buf: &mut [u8]) -> IoResult<usize>;

    async fn read_exact(&self, mut buf: &mut [u8]) -> IoResult<()> {
        let mut left = buf.len();
        while left > 0 {
            let newly_read = self.read(buf).await?;
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

#[async_trait]
pub trait Write {
    async fn write(&self, buf: &[u8]) -> IoResult<usize>;

    async fn flush(&self) -> IoResult<()> {
        Ok(())
    }

    async fn write_all(&self, mut buf: &[u8]) -> IoResult<()> {
        let mut left = buf.len();
        while left > 0 {
            let newly_written = self.write(buf).await?;
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

#[async_trait]
pub trait Control {
    async fn call(&self, msg: FramebufferCM) -> IoResult<()>;
}

pub struct FileInterface {
    pub sync_read: Option<&'static (dyn SyncRead + Send + Sync)>,
    pub read: Option<&'static (dyn Read + Send + Sync)>,
    pub sync_write: Option<&'static (dyn SyncWrite + Send + Sync)>,
    pub write: Option<&'static (dyn Write + Send + Sync)>,
    pub ctrl: Option<&'static (dyn Control + Send + Sync)>,
}
