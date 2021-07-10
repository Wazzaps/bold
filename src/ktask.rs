use alloc::boxed::Box;
use alloc::collections::VecDeque;
use core::ptr::null;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use core::{future::Future, pin::Pin};
use spin::Mutex;

pub(crate) static EXECUTOR: Mutex<Option<SimpleExecutor>> = Mutex::new(None);

pub struct Task {
    future: Pin<Box<(dyn Future<Output = ()> + Send)>>,
}

impl Task {
    #[allow(dead_code)]
    pub fn new(future: impl Future<Output = ()> + Send + 'static) -> Task {
        Task {
            future: Box::pin(future),
        }
    }

    pub fn new_raw(future: Pin<Box<(dyn Future<Output = ()> + Send)>>) -> Task {
        Task { future }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task)
    }

    pub fn run(&mut self) {
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {} // task done
                Poll::Pending => self.task_queue.push_back(task),
            }
        }
    }

    #[allow(dead_code)]
    pub fn run_blocking(task: Task) {
        let mut executor = SimpleExecutor::new();
        executor.spawn(task);
        executor.run();
    }
}

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(null(), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

#[inline]
pub async fn yield_now() {
    YieldNow(false).await
}

struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    // The futures executor is implemented as a FIFO queue, so all this future
    // does is re-schedule the future back to the end of the queue, giving room
    // for other futures to progress.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.0 {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

pub fn init() {
    EXECUTOR.lock().replace(SimpleExecutor::new());
}

pub fn run() {
    EXECUTOR.lock().as_mut().unwrap().run();
}

#[macro_export]
macro_rules! spawn_task {
    ($b:block) => {{
        let mut executor = crate::ktask::EXECUTOR.lock();
        let executor = executor.as_mut().unwrap();
        let closure = async move || ($b);
        executor.spawn(crate::ktask::Task::new_raw(Box::pin(closure())));
    }};
}
