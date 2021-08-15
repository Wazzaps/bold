// pub(crate) mod condvar;
pub(crate) mod dir;
pub(crate) mod signal;
pub(crate) mod spsc_mux;
pub(crate) mod spsc_queue;
pub(crate) mod well_known;

use crate::prelude::*;
use core::fmt::{Debug, Formatter};
pub use dir::IpcDir;
use futures::stream::BoxStream;
use futures::StreamExt;
use spin::RwLock;
pub use spsc_queue::IpcSpscQueue;

#[derive(Clone)]
pub struct IpcRef {
    pub id: u64,
    pub inner: Arc<dyn IpcNode + Send + Sync>,
}

impl Debug for IpcRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        display_bstr(f, &self.inner.describe())?;
        write!(f, " {:x}", self.id)
    }
}

#[async_trait]
pub trait IpcNode {
    fn dir_list<'a>(self: Arc<Self>) -> Option<BoxStream<'a, IpcRef>>;
    async fn dir_get(self: Arc<Self>, id: u64) -> Option<IpcRef>;
    async fn dir_create(self: Arc<Self>, id: u64) -> Option<IpcRef>;
    async fn dir_link(
        self: Arc<Self>,
        id: u64,
        node: Arc<dyn IpcNode + Send + Sync>,
    ) -> Option<IpcRef>;
    fn queue_write(self: Arc<Self>, data: &[u8]) -> Result<usize, ()>;
    async fn queue_read(self: Arc<Self>, dest: &mut [u8]) -> Option<usize>;
    fn describe(&self) -> [u8; 4];
}

impl IpcRef {
    pub fn dir_list(&self) -> Option<BoxStream<IpcRef>> {
        self.inner.clone().dir_list()
    }

    pub async fn dir_get(&self, id: u64) -> Option<IpcRef> {
        self.inner.clone().dir_get(id).await
    }

    pub async fn dir_create(&self, id: u64) -> Option<IpcRef> {
        self.inner.clone().dir_create(id).await
    }

    pub async fn dir_link(&self, id: u64, node: Arc<dyn IpcNode + Send + Sync>) -> Option<IpcRef> {
        self.inner.clone().dir_link(id, node).await
    }

    pub fn queue_write(&self, data: &[u8]) -> Result<usize, ()> {
        self.inner.clone().queue_write(data)
    }

    pub async fn queue_write_all(&self, mut data: &[u8]) -> Result<(), ()> {
        let mut left = data.len();
        while left > 0 {
            let newly_written = self.queue_write(data)?;
            if newly_written == 0 {
                // EOF
                return Err(());
            }
            data = &data[newly_written..];
            left -= newly_written;
        }
        Ok(())
    }

    pub async fn queue_read(&self, dest: &mut [u8]) -> Option<usize> {
        self.inner.clone().queue_read(dest).await
    }

    pub fn describe(&self) -> [u8; 4] {
        self.inner.describe()
    }
}

pub fn init() {
    ROOT.write().replace(IpcRef {
        id: well_known::ROOT,
        inner: IpcDir::new_filled(vec![
            IpcRef {
                id: well_known::ROOT_EXAMPLE,
                inner: IpcDir::new_filled(vec![
                    IpcRef {
                        id: well_known::EXAMPLE_1,
                        inner: IpcDir::new_empty(),
                    },
                    IpcRef {
                        id: well_known::EXAMPLE_2,
                        inner: IpcDir::new_empty(),
                    },
                    IpcRef {
                        id: well_known::EXAMPLE_3,
                        inner: IpcSpscQueue::new(),
                    },
                ]),
            },
            IpcRef {
                id: well_known::ROOT_DEVICES,
                inner: IpcDir::new_filled(vec![
                    IpcRef {
                        id: well_known::DEVICES_RPI_UART,
                        inner: IpcDir::new_filled(vec![IpcRef {
                            id: well_known::RPI_UART1,
                            inner: IpcDir::new_empty(),
                        }]),
                    },
                    IpcRef {
                        id: well_known::DEVICES_RPI_FB_CON,
                        inner: IpcDir::new_filled(vec![IpcRef {
                            id: well_known::RPI_FB_CON0,
                            inner: IpcDir::new_empty(),
                        }]),
                    },
                ]),
            },
        ]),
    });
}

pub async fn test() {
    let ipc_dir = ROOT.read().as_ref().unwrap().clone();

    // Add directory using `dir_create`
    ipc_dir.dir_create(0xdeadbeef).await.unwrap();

    // List root recursively
    let mut stream = ipc_dir.dir_list().unwrap();
    while let Some(item) = stream.next().await {
        println!("- {:?}", item);
        let mut inner_stream = item.dir_list().unwrap();
        while let Some(inner_item) = inner_stream.next().await {
            println!("  - {:?}", inner_item);
        }
    }

    let dir1 = ipc_dir.dir_get(1).await.unwrap();
    println!("dir_get(1) = {:?}", dir1);

    // Write to spsc
    {
        let queue = dir1.dir_get(3).await.unwrap();
        queue.queue_write(b"Hello").unwrap();
    }

    // Read from spsc
    {
        let queue = dir1.dir_get(3).await.unwrap();
        let mut res = [0u8; 5];
        assert_eq!(queue.queue_read(&mut res).await.unwrap(), 5);
        assert_eq!(&res, b"Hello");
    }

    // Test spsc
    // let mut q = SpscQueue::<512>::new();
    // assert_eq!(q.read(1), b"");
    // assert_eq!(q.read(128), b"");
    // assert_eq!(q.read(256), b"");
    // assert_eq!(q.write(b"abcd"), 4);
    // assert_eq!(q.write(b"abcd"), 4);
    // assert_eq!(q.read(8), b"abcdabcd");
    //
    // let s_128 = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    // let s_120 = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    // let s_8 = b"aaaaaaaa";
    // assert_eq!(q.write(s_128), 128);
    // assert_eq!(q.read(128), s_120);
    // assert_eq!(q.read(128), s_8);
}

pub static ROOT: RwLock<Option<IpcRef>> = RwLock::new(None);
