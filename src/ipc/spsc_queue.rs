use crate::ipc::{IpcNode, IpcRef};

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use async_trait::async_trait;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use futures::prelude::stream::BoxStream;
use spin::Mutex;

struct SpscReadableWaiter<'a>(&'a Mutex<Box<SpscQueue<512>>>);

impl<'a> Future for SpscReadableWaiter<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut spsc = self.0.lock();
        if spsc.is_readable() {
            Poll::Ready(())
        } else {
            spsc.wait_readable(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct SpscQueue<const S: usize> {
    buffer: [u8; S],
    read_head: usize,
    write_head: usize,
    read_waiters: Vec<Waker>,
    write_waiters: Vec<Waker>,
}

// FIXME: Can be made atomic I think?
impl<const S: usize> SpscQueue<S> {
    pub fn new() -> Self {
        Self {
            buffer: [0; S],
            read_head: 0,
            write_head: 0,
            read_waiters: Vec::new(),
            write_waiters: Vec::new(),
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

            // Wake readers
            for reader_waker in self.read_waiters.iter() {
                reader_waker.wake_by_ref();
            }
            self.read_waiters.clear();
        }
        write_size
    }

    pub fn read(&mut self, amount: usize) -> &[u8] {
        let next_split = (self.read_head / S * S) + S;
        let read_head = self.read_head;
        let read_target = self.write_head.min(self.read_head + amount);
        let response = if read_target > next_split {
            // Need to split the reads, return the first one
            self.read_head = next_split;
            &self.buffer[read_head % S..S]
        } else {
            // Fits in single read
            self.read_head = read_target;
            &self.buffer[read_head % S..(read_head % S) + read_target - read_head]
        };

        // Wake writers
        if !response.is_empty() {
            for writer_waker in self.write_waiters.iter() {
                writer_waker.wake_by_ref();
            }
            self.write_waiters.clear();
        }

        response
    }

    pub fn is_readable(&self) -> bool {
        self.read_head != self.write_head
    }

    pub fn wait_readable(&mut self, x: Waker) {
        self.read_waiters.push(x);
    }
}

pub struct IpcSpscQueue {
    queue: Mutex<Box<SpscQueue<512>>>,
}

impl IpcSpscQueue {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            queue: Mutex::new(Box::new(SpscQueue::<512>::new())),
        })
    }
}

#[async_trait]
impl IpcNode for IpcSpscQueue {
    fn dir_list<'a>(self: Arc<Self>) -> Option<BoxStream<'a, IpcRef>> {
        None
    }

    async fn dir_get(self: Arc<Self>, _id: u64) -> Option<IpcRef> {
        None
    }

    async fn dir_create(self: Arc<Self>, _id: u64) -> Option<IpcRef> {
        None
    }

    async fn dir_link(
        self: Arc<Self>,
        _id: u64,
        _node: Arc<dyn IpcNode + Send + Sync>,
    ) -> Option<IpcRef> {
        None
    }

    fn queue_write(self: Arc<Self>, data: &[u8]) -> Result<usize, ()> {
        Ok(self.queue.lock().write(data))
    }

    async fn queue_read(self: Arc<Self>, dest: &mut [u8]) -> Option<usize> {
        loop {
            {
                let mut queue = self.queue.lock();
                let result = queue.read(dest.len());
                (&mut dest[..result.len()]).copy_from_slice(result);

                if !result.is_empty() {
                    return Some(result.len());
                }
            }

            SpscReadableWaiter(&self.queue).await;
        }
    }

    fn describe(&self) -> [u8; 4] {
        *b"SPSC"
    }
}
