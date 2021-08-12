use crate::ipc::{IpcRef, IpcSpscQueue};
use crate::ktask;
use crate::spawn_task;
use alloc::boxed::Box;

// TODO: make destructible
pub fn mux_from_input(input: IpcRef) -> (IpcRef, IpcRef) {
    let out_queue_1 = IpcRef {
        id: 0,
        inner: IpcSpscQueue::new(),
    };
    let out_queue_2 = IpcRef {
        id: 0,
        inner: IpcSpscQueue::new(),
    };

    let copies = (out_queue_1.clone(), out_queue_2.clone());
    mux_with(input, out_queue_1, out_queue_2);

    copies
}

pub fn mux_into_outputs(out_queue_1: IpcRef, out_queue_2: IpcRef) -> IpcRef {
    let input = IpcRef {
        id: 0,
        inner: IpcSpscQueue::new(),
    };

    let copy = input.clone();
    mux_with(input, out_queue_1, out_queue_2);

    copy
}

pub fn mux_with(input: IpcRef, out_queue_1: IpcRef, out_queue_2: IpcRef) {
    spawn_task!(b"StreamMux", {
        let mut buf = [0u8; 256];
        loop {
            if let Some(data_len) = input.queue_read(&mut buf).await {
                out_queue_1.queue_write_all(&buf[..data_len]).await.unwrap();
                out_queue_2.queue_write_all(&buf[..data_len]).await.unwrap();
            }
            ktask::yield_now().await;
        }
    });
}
