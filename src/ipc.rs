use crate::println;
use crate::unwrap_variant;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};
use futures::stream::BoxStream;
use futures::{stream, StreamExt};
use spin::{Mutex, RwLock};

#[derive(Clone)]
pub struct IpcRef {
    pub id: u64,
    pub inner: Arc<IpcNode>,
}

impl Debug for IpcRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self.inner.as_ref() {
            IpcNode::Dir(_) => write!(f, ":{:x}:Dir", self.id),
            IpcNode::SpscQueue(_) => write!(f, ":{:x}:SpscQueue", self.id),
            IpcNode::SpmcQueue => write!(f, ":{:x}:SpmcQueue", self.id),
            IpcNode::MpscQueue => write!(f, ":{:x}:MpscQueue", self.id),
            IpcNode::MpmcQueue => write!(f, ":{:x}:MpmcQueue", self.id),
            IpcNode::Blob => write!(f, ":{:x}:Blob", self.id),
            IpcNode::Endpoint => write!(f, ":{:x}:Endpoint", self.id),
        }
    }
}

pub enum IpcNode {
    Dir(IpcDir),
    SpscQueue(IpcSpscQueue),
    SpmcQueue,
    MpscQueue,
    MpmcQueue,
    Blob,
    Endpoint,
}

pub struct IpcDir {
    entries: RwLock<Vec<IpcRef>>,
}

pub struct SpscQueue<const S: usize> {
    buffer: [u8; S],
    read_head: usize,
    write_head: usize,
}

// FIXME: Can be made atomic I think?
impl<const S: usize> SpscQueue<S> {
    pub fn new() -> Self {
        Self {
            buffer: [0; S],
            read_head: 0,
            write_head: 0,
        }
    }
    pub fn write(&mut self, data: &[u8]) -> usize {
        let next_split = (self.write_head / S * S) + S;
        let available_space = S - (self.write_head - self.read_head);
        let write_size = data.len().min(available_space);
        if write_size > 0 {
            if self.write_head + write_size > next_split {
                // Need to split the writes
                let cutoff = next_split - self.write_head;
                (&mut self.buffer[self.write_head % S..S]).copy_from_slice(&data[..cutoff]);
                (&mut self.buffer[..write_size - cutoff])
                    .copy_from_slice(&data[cutoff..write_size]);
            } else {
                // Fits in single write
                (&mut self.buffer[self.write_head % S..self.write_head % S + write_size])
                    .copy_from_slice(&data[..write_size]);
            }
            self.write_head += write_size;
        }
        write_size
    }

    pub fn read(&mut self, amount: usize) -> &[u8] {
        let next_split = (self.read_head / S * S) + S;
        let read_head = self.read_head;
        let read_target = self.write_head.min(self.read_head + amount);
        if read_target > next_split {
            // Need to split the reads, return the first one
            self.read_head = next_split;
            &self.buffer[read_head % S..S]
        } else {
            // Fits in single read
            self.read_head = read_target;
            &self.buffer[read_head % S..(read_head % S) + read_target - read_head]
        }
    }
}

pub struct IpcSpscQueue {
    queue: Mutex<Box<SpscQueue<512>>>,
}

impl IpcSpscQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Box::new(SpscQueue::<512>::new())),
        }
    }
}

impl IpcRef {
    pub fn dir_list(&self) -> Option<BoxStream<IpcRef>> {
        if !matches!(self.inner.as_ref(), IpcNode::Dir(_)) {
            return None;
        }
        Some(Box::pin(stream::unfold(0, move |idx| {
            let node = self.inner.clone(); // Eww

            async move {
                let dir = unwrap_variant!(node.as_ref(), IpcNode::Dir);
                let entries = dir.entries.read();

                if idx < entries.len() {
                    Some((
                        // Yield this
                        entries[idx].clone(),
                        // Next state
                        idx + 1,
                    ))
                } else {
                    None
                }
            }
        })))
    }

    pub async fn dir_get(&self, id: u64) -> Option<IpcRef> {
        if !matches!(self.inner.as_ref(), IpcNode::Dir(_)) {
            return None;
        }

        let dir = unwrap_variant!(self.inner.as_ref(), IpcNode::Dir);
        let entries = dir.entries.read();
        entries.iter().find(|e| e.id == id).cloned()
    }

    pub async fn dir_create(&self, id: u64) -> Option<IpcRef> {
        self.dir_link(
            id,
            Arc::new(IpcNode::Dir(IpcDir {
                entries: RwLock::new(vec![]),
            })),
        )
        .await
    }

    pub async fn dir_link(&self, id: u64, node: Arc<IpcNode>) -> Option<IpcRef> {
        if !matches!(self.inner.as_ref(), IpcNode::Dir(_)) {
            return None;
        }

        let dir = unwrap_variant!(self.inner.as_ref(), IpcNode::Dir);
        let mut entries = dir.entries.write();
        for entry in entries.iter() {
            if entry.id == id {
                return None;
            }
        }
        let new_ent = IpcRef { id, inner: node };
        entries.push(new_ent.clone());

        Some(new_ent)
    }

    pub async fn queue_write(&self, data: &[u8]) -> Result<usize, ()> {
        match self.inner.as_ref() {
            IpcNode::SpscQueue(q) => Ok(q.queue.lock().write(data)),
            // IpcNode::SpmcQueue => {}
            // IpcNode::MpscQueue => {}
            // IpcNode::MpmcQueue => {}
            _ => Err(()),
        }
    }

    pub async fn queue_write_all(&self, mut data: &[u8]) -> Result<(), ()> {
        let mut left = data.len();
        while left > 0 {
            let newly_written = self.queue_write(data).await?;
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
        match self.inner.as_ref() {
            IpcNode::SpscQueue(q) => {
                let mut queue = q.queue.lock();
                let result = queue.read(dest.len());
                (&mut dest[..result.len()]).copy_from_slice(result);
                Some(result.len())
            }
            // IpcNode::SpmcQueue => {}
            // IpcNode::MpscQueue => {}
            // IpcNode::MpmcQueue => {}
            _ => None,
        }
    }
}

pub fn init() {
    ROOT.write().replace(IpcRef {
        id: 0,
        inner: Arc::new(IpcNode::Dir(IpcDir {
            entries: RwLock::new(vec![
                IpcRef {
                    id: 1,
                    inner: Arc::new(IpcNode::Dir(IpcDir {
                        entries: RwLock::new(vec![
                            IpcRef {
                                id: 1,
                                inner: Arc::new(IpcNode::Dir(IpcDir {
                                    entries: RwLock::new(vec![]),
                                })),
                            },
                            IpcRef {
                                id: 2,
                                inner: Arc::new(IpcNode::Dir(IpcDir {
                                    entries: RwLock::new(vec![]),
                                })),
                            },
                            IpcRef {
                                id: 3,
                                inner: Arc::new(IpcNode::SpscQueue(IpcSpscQueue::new())),
                            },
                        ]),
                    })),
                },
                IpcRef {
                    id: 2,
                    inner: Arc::new(IpcNode::Dir(IpcDir {
                        entries: RwLock::new(vec![]),
                    })),
                },
                IpcRef {
                    id: 3,
                    inner: Arc::new(IpcNode::Dir(IpcDir {
                        entries: RwLock::new(vec![]),
                    })),
                },
                IpcRef {
                    id: 4,
                    inner: Arc::new(IpcNode::Dir(IpcDir {
                        entries: RwLock::new(vec![]),
                    })),
                },
            ]),
        })),
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
        queue.queue_write(b"Hello").await.unwrap();
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
