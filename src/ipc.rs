use crate::println;
use crate::unwrap_variant;
use alloc::boxed::Box;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};
use core::future::Future;
use core::pin::Pin;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::{stream, Stream, StreamExt};
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
            IpcNode::Stream => write!(f, "IpcNode::Stream#{}", self.id),
            IpcNode::Blob => write!(f, "IpcNode::Blob#{}", self.id),
            IpcNode::Endpoint => write!(f, "IpcNode::Endpoint#{}", self.id),
        }
    }
}

enum IpcNode {
    Dir(IpcDir),
    Stream,
    Blob,
    Endpoint,
}

struct IpcDir {
    entries: RwLock<Vec<IpcRef>>,
}

impl IpcNode {
    pub fn dir_list<'a>(this: Arc<IpcNode>) -> Option<BoxStream<'a, IpcRef>> {
        if !matches!(this.as_ref(), IpcNode::Dir(_)) {
            return None;
        }
        Some(Box::pin(stream::unfold(0, move |idx| {
            let this = this.clone(); // Eww

            async move {
                let this = unwrap_variant!(this.as_ref(), IpcNode::Dir);
                let entries = this.entries.read();

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

    pub fn dir_get<'a>(this: Arc<IpcNode>, id: u64) -> BoxFuture<'a, Option<IpcRef>> {
        if !matches!(this.as_ref(), IpcNode::Dir(_)) {
            return Box::pin(async { None });
        }
        Box::pin(async move {
            let this = unwrap_variant!(this.as_ref(), IpcNode::Dir);
            let entries = this.entries.read();

            entries.iter().find(|e| e.id == id).cloned()
        })
    }
}

pub fn init() {
    ROOT.write().replace(Arc::new(IpcNode::Dir(IpcDir {
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
    })));
}

pub async fn test() {
    let ipc_dir = ROOT.read().as_ref().unwrap().clone();
    let mut stream = IpcNode::dir_list(ipc_dir.clone()).unwrap();
    while let Some(item) = stream.next().await {
        println!("{:?}", item);
        let mut inner_stream = IpcNode::dir_list(item.inner.clone()).unwrap();
        while let Some(inner_item) = inner_stream.next().await {
            println!("- {:?}", inner_item);
        }
    }
    println!("{:?}", IpcNode::dir_get(ipc_dir, 0).await.unwrap());
}

static ROOT: RwLock<Option<Arc<IpcNode>>> = RwLock::new(None);
