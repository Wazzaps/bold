use crate::println;
use crate::unwrap_variant;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::{stream, StreamExt};
use spin::RwLock;

#[derive(Clone)]
struct IpcRef {
    id: u64,
    inner: Arc<IpcNode>,
}

impl Debug for IpcRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self.inner.as_ref() {
            IpcNode::Dir(_) => write!(f, "IpcNode::Dir#{}", self.id),
            IpcNode::Pipe => write!(f, "IpcNode::Pipe#{}", self.id),
            IpcNode::Blob => write!(f, "IpcNode::Blob#{}", self.id),
            IpcNode::Endpoint => write!(f, "IpcNode::Endpoint#{}", self.id),
        }
    }
}

enum IpcNode {
    Dir(IpcDir),
    Pipe,
    Blob,
    Endpoint,
}

struct IpcDir {
    entries: RwLock<Vec<IpcRef>>,
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

    pub fn dir_get(&self, id: u64) -> BoxFuture<Option<IpcRef>> {
        if !matches!(self.inner.as_ref(), IpcNode::Dir(_)) {
            return Box::pin(async { None });
        }
        Box::pin(async move {
            let dir = unwrap_variant!(self.inner.as_ref(), IpcNode::Dir);
            let entries = dir.entries.read();
            entries.iter().find(|e| e.id == id).cloned()
        })
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
                                inner: Arc::new(IpcNode::Dir(IpcDir {
                                    entries: RwLock::new(vec![]),
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
        println!("{:?}", item);
        let mut inner_stream = item.dir_list().unwrap();
        while let Some(inner_item) = inner_stream.next().await {
            println!("- {:?}", inner_item);
        }
    }
    println!("{:?}", ipc_dir.dir_get(1).await.unwrap());
}

static ROOT: RwLock<Option<IpcRef>> = RwLock::new(None);
