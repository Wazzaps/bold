use crate::ipc::{IpcRef, IpcSpscQueue};
use crate::ktask;
use crate::spawn_task;
use alloc::boxed::Box;

// TODO: make destructible
pub fn mux(input: IpcRef) -> (IpcRef, IpcRef) {
    let out_queue_1 = IpcRef {
        id: 0,
        inner: IpcSpscQueue::new(),
    };
    let out_queue_2 = IpcRef {
        id: 0,
        inner: IpcSpscQueue::new(),
    };
    let copies = (out_queue_1.clone(), out_queue_2.clone());

    spawn_task!({
        let mut buf = [0u8; 1];
        loop {
            if let Some(1) = input.queue_read(&mut buf).await {
                out_queue_1.queue_write(&buf).await.unwrap();
                out_queue_2.queue_write(&buf).await.unwrap();
            }
            ktask::yield_now().await;
        }
    });

    copies
}
