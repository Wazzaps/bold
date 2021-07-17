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
struct IpcRef {
    id: u64,
    inner: Arc<IpcNode>,
}

impl Debug for IpcRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self.inner.as_ref() {
            IpcNode::Dir(_) => write!(f, "IpcNode::Dir#{}", self.id),
            IpcNode::SpscQueue(_) => write!(f, "IpcNode::SpscQueue#{}", self.id),
            IpcNode::SpmcQueue => write!(f, "IpcNode::SpmcQueue#{}", self.id),
            IpcNode::MpscQueue => write!(f, "IpcNode::MpscQueue#{}", self.id),
            IpcNode::MpmcQueue => write!(f, "IpcNode::MpmcQueue#{}", self.id),
            IpcNode::Blob => write!(f, "IpcNode::Blob#{}", self.id),
            IpcNode::Endpoint => write!(f, "IpcNode::Endpoint#{}", self.id),
        }
    }
}

enum IpcNode {
    Dir(IpcDir),
    SpscQueue(IpcSpscQueue),
    SpmcQueue,
    MpscQueue,
    MpmcQueue,
    Blob,
    Endpoint,
}

struct IpcDir {
    entries: RwLock<Vec<IpcRef>>,
}

struct SpscQueue<const S: usize> {
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
                (&mut self.buffer[self.write_head..self.write_head + write_size])
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
            &self.buffer[read_head % S..read_target % S]
        }
    }
}

struct IpcSpscQueue {
    queue: Mutex<Box<SpscQueue<128>>>,
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
        let new_ent = IpcRef {
            id,
            inner: Arc::new(IpcNode::Dir(IpcDir {
                entries: RwLock::new(vec![]),
            })),
        };
        entries.push(new_ent.clone());

        Some(new_ent)
    }

    pub async fn queue_write(&self, data: &[u8]) -> Option<usize> {
        match self.inner.as_ref() {
            IpcNode::SpscQueue(q) => Some(q.queue.lock().write(data)),
            // IpcNode::SpmcQueue => {}
            // IpcNode::MpscQueue => {}
            // IpcNode::MpmcQueue => {}
            _ => None,
        }
    }

    pub async fn queue_read(&self, amount: usize) -> Option<Vec<u8>> {
        match self.inner.as_ref() {
            IpcNode::SpscQueue(q) => Some(q.queue.lock().read(amount).to_vec()),
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
                                inner: Arc::new(IpcNode::SpscQueue(IpcSpscQueue {
                                    queue: Mutex::new(Box::new(SpscQueue::<128>::new())),
                                })),
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
        assert_eq!(queue.queue_read(5).await.unwrap(), b"Hello");
    }

    // Test spsc
    // let mut q = SpscQueue::<128>::new();
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

static ROOT: RwLock<Option<IpcRef>> = RwLock::new(None);
