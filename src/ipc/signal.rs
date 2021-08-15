use crate::prelude::*;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use spin::Mutex;

pub struct Signal {
    waiters: Mutex<VecDeque<Waker>>,
}

impl Signal {
    pub fn new() -> Signal {
        Signal {
            waiters: Mutex::new(VecDeque::new()),
        }
    }

    pub async fn wait(&self) {
        struct YieldWaiter<'a> {
            state: bool,
            wakers: &'a Mutex<VecDeque<Waker>>,
        }

        impl<'a> Future for YieldWaiter<'a> {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if !self.state {
                    self.state = true;

                    self.wakers.lock().push_back(cx.waker().clone());
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            }
        }

        YieldWaiter {
            state: false,
            wakers: &self.waiters,
        }
        .await;
    }

    pub fn notify_all(&self) {
        let mut waiters = self.waiters.lock();
        waiters.iter().for_each(Waker::wake_by_ref);
        waiters.clear();
    }

    pub fn notify_one(&self) {
        if let Some(w) = self.waiters.lock().pop_front() {
            w.wake_by_ref()
        }
    }
}

// struct SpscReadableWaiter<'a>(&'a Mutex<Box<SpscQueue<512>>>);
//
// impl<'a> Future for SpscReadableWaiter<'a> {
//     type Output = ();
//
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         let mut spsc = self.0.lock();
//         if spsc.is_readable() {
//             Poll::Ready(())
//         } else {
//             spsc.wait_readable(cx.waker().clone());
//             Poll::Pending
//         }
//     }
// }
