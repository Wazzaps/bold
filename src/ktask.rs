use crate::arch::aarch64::mmio::get_uptime_us;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use core::ptr::null;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use core::{future::Future, pin::Pin};
use spin::{Mutex, Once};

pub(crate) static EXECUTOR: Once<SimpleExecutor> = Once::new();
static PERF_INFO: Mutex<PerfInfo> = Mutex::new(PerfInfo::new());

pub struct PerfInfo {
    pub boot_time_us: u64,
    pub cpu_time_us: u64,
    pub total_yields: u64,
    pub tasks_spawned: u64,
    pub tasks_killed: u64,
}

#[derive(Debug)]
pub struct PerfReport {
    uptime_us: u64,
    cpu_time_us: u64,
    total_yields: u64,
    avg_time_between_yields_us: u64,
    tasks_spawned: u64,
    tasks_killed: u64,
    current_tasks: u64,
}

impl PerfInfo {
    const fn new() -> Self {
        Self {
            boot_time_us: 0,
            cpu_time_us: 0,
            total_yields: 0,
            tasks_spawned: 0,
            tasks_killed: 0,
        }
    }

    fn report(&self) -> PerfReport {
        let avg_time_between_yields_us =
            self.cpu_time_us.checked_div(self.total_yields).unwrap_or(0);
        PerfReport {
            uptime_us: get_uptime_us() - self.boot_time_us,
            cpu_time_us: self.cpu_time_us,
            total_yields: self.total_yields,
            avg_time_between_yields_us,
            tasks_spawned: self.tasks_spawned,
            tasks_killed: self.tasks_killed,
            current_tasks: self.tasks_spawned - self.tasks_killed,
        }
    }
}

pub struct Task {
    future: Pin<Box<(dyn Future<Output = ()> + Send)>>,
}

impl Task {
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
    task_queue: Mutex<VecDeque<Task>>,
}

impl SimpleExecutor {
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn spawn(&self, task: Task) {
        self.task_queue.lock().push_back(task);
        let mut perf_info = PERF_INFO.lock();
        perf_info.tasks_spawned += 1;
    }

    pub fn run(&self) {
        loop {
            let next_task = self.task_queue.lock().pop_front();
            if let Some(mut task) = next_task {
                let waker = dummy_waker();
                let mut context = Context::from_waker(&waker);

                let uptime_before = get_uptime_us();
                let poll_result = task.poll(&mut context);
                let uptime_after = get_uptime_us();

                let mut perf_info = PERF_INFO.lock();
                match poll_result {
                    Poll::Ready(()) => {
                        // task done
                        perf_info.tasks_killed += 1;
                    }
                    Poll::Pending => {
                        self.task_queue.lock().push_back(task);
                    }
                }
                perf_info.total_yields += 1;
                perf_info.cpu_time_us += uptime_after - uptime_before;
            } else {
                break;
            }
        }
    }

    pub fn run_blocking(task: Task) {
        let executor = SimpleExecutor::new();
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
    EXECUTOR.call_once(SimpleExecutor::new);
    let mut perf_info = PERF_INFO.lock();
    perf_info.boot_time_us = get_uptime_us();
}

pub fn run() {
    EXECUTOR.wait().unwrap().run();
}

pub fn perf_report() -> PerfReport {
    PERF_INFO.lock().report()
}

#[macro_export]
macro_rules! spawn_task {
    ($b:block) => {{
        let executor = crate::ktask::EXECUTOR.wait().unwrap();
        let closure = async move || ($b);
        executor.spawn(crate::ktask::Task::new_raw(Box::pin(closure())));
    }};
}
