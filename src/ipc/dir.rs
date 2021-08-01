use crate::ipc::{IpcNode, IpcRef};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use async_trait::async_trait;
use futures::prelude::stream::BoxStream;
use futures::stream;
use spin::RwLock;

pub struct IpcDir {
    entries: RwLock<Vec<IpcRef>>,
}

impl IpcDir {
    pub fn new_empty() -> Arc<Self> {
        IpcDir::new_filled(vec![])
    }
    pub fn new_filled(entries: Vec<IpcRef>) -> Arc<Self> {
        Arc::new(IpcDir {
            entries: RwLock::new(entries),
        })
    }
}

#[async_trait]
impl IpcNode for IpcDir {
    fn dir_list<'a>(self: Arc<Self>) -> Option<BoxStream<'a, IpcRef>> {
        Some(Box::pin(stream::unfold(0, move |idx| {
            let node = self.clone(); // Eww

            async move {
                let entries = node.entries.read();

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

    async fn dir_get(self: Arc<Self>, id: u64) -> Option<IpcRef> {
        let entries = self.entries.read();
        entries.iter().find(|e| e.id == id).cloned()
    }

    async fn dir_create(self: Arc<Self>, id: u64) -> Option<IpcRef> {
        self.dir_link(id, IpcDir::new_empty()).await
    }

    async fn dir_link(
        self: Arc<Self>,
        id: u64,
        node: Arc<dyn IpcNode + Send + Sync>,
    ) -> Option<IpcRef> {
        let mut entries = self.entries.write();
        for entry in entries.iter() {
            if entry.id == id {
                return None;
            }
        }
        let new_ent = IpcRef { id, inner: node };
        entries.push(new_ent.clone());

        Some(new_ent)
    }

    fn queue_write(self: Arc<Self>, _data: &[u8]) -> Result<usize, ()> {
        Err(())
    }

    async fn queue_read(self: Arc<Self>, _dest: &mut [u8]) -> Option<usize> {
        None
    }

    fn describe(&self) -> [u8; 4] {
        *b"DIR "
    }
}
