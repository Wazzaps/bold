use crate::println;
use alloc::boxed::Box;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};
use core::pin::Pin;
use futures::stream::BoxStream;
use futures::{stream, Stream, StreamExt};
use spin::RwLock;

struct IpcRef {
    id: u64,
    inner: Weak<Node>,
}

impl Debug for IpcRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "IpcRef(id: {})", self.id)
    }
}

enum Node {
    IpcDir(IpcDir),
    IpcStream,
    IpcBlob,
}

struct IpcDir {
    example_count: usize,
    // entries: RwLock<Vec<IpcRef>>
}

impl Node {
    pub fn list<'a>(this: Arc<Node>) -> BoxStream<'a, IpcRef> {
        Box::pin(stream::unfold(0, move |id| {
            let this = this.clone(); // Eww

            async move {
                let this = if let Node::IpcDir(this) = this.as_ref() {
                    this
                } else {
                    panic!();
                };

                if id < this.example_count {
                    Some((
                        // Yield this
                        IpcRef {
                            id: id as u64,
                            inner: Weak::new(),
                        },
                        // Next state
                        id + 1,
                    ))
                } else {
                    None
                }
            }
        }))
    }
}

pub fn init() {
    ROOT.write().replace(Arc::new(Node::IpcDir(IpcDir {
        // entries: RwLock::new(vec![]),
        example_count: 6,
    })));
}

pub async fn test() {
    let ipc_dir = ROOT.read().as_ref().unwrap().clone();
    let mut s = Node::list(ipc_dir);
    while let Some(item) = s.next().await {
        println!("{:?}", item);
    }
}

static ROOT: RwLock<Option<Arc<Node>>> = RwLock::new(None);
