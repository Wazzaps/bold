use crate::arch::aarch64::mmio::get_uptime_us;
use crate::prelude::*;

use crate::threads;
use core::ptr::null;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use core::{future::Future, pin::Pin};
use spin::{Mutex, Once, RwLock};

pub(crate) static EXECUTOR: Once<SimpleExecutor> = Once::new();
static PERF_INFO: Mutex<PerfInfo> = Mutex::new(PerfInfo::new());
static PID_COUNTER: AtomicUsize = AtomicUsize::new(1);

pub struct PerfInfo {
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
            uptime_us: get_uptime_us(),
            cpu_time_us: self.cpu_time_us,
            total_yields: self.total_yields,
            avg_time_between_yields_us,
            tasks_spawned: self.tasks_spawned,
            tasks_killed: self.tasks_killed,
            current_tasks: self.tasks_spawned - self.tasks_killed,
        }
    }
}

pub struct TaskPerfInfo {
    pub id: usize,
    pub name: &'static [u8],
    pub uptime_us: u64,
    pub cpu_time_us: u64,
    pub total_yields: u64,
}

pub struct Task {
    id: usize,
    name: &'static [u8],
    start_time_us: u64,
    cpu_time_us: u64,
    total_yields: u64,
    future: Mutex<Pin<Box<(dyn Future<Output = ()> + Send)>>>,
}

impl Task {
    pub fn new(name: &'static [u8], future: impl Future<Output = ()> + Send + 'static) -> Task {
        Task {
            id: PID_COUNTER.fetch_add(1, Ordering::SeqCst),
            name,
            start_time_us: get_uptime_us(),
            cpu_time_us: 0,
            total_yields: 0,
            future: Mutex::new(Box::pin(future)),
        }
    }

    pub fn new_raw(
        name: &'static [u8],
        future: Pin<Box<(dyn Future<Output = ()> + Send)>>,
    ) -> Task {
        Task {
            id: PID_COUNTER.fetch_add(1, Ordering::SeqCst),
            name,
            start_time_us: get_uptime_us(),
            cpu_time_us: 0,
            total_yields: 0,
            future: Mutex::new(future),
        }
    }

    fn poll(&self, context: &mut Context) -> Poll<()> {
        self.future.lock().as_mut().poll(context)
    }
}

pub struct SimpleExecutor {
    tasks: Mutex<Vec<Arc<RwLock<Task>>>>,
    run_queue: Mutex<VecDeque<usize>>,
}

impl SimpleExecutor {
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            tasks: Mutex::new(Vec::new()),
            run_queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn spawn(&self, task: Task) {
        let id = task.id;
        self.tasks.lock().push(Arc::new(RwLock::new(task)));

        {
            let _locked = irq_lock();
            self.run_queue.lock().push_back(id);
        }

        let mut perf_info = PERF_INFO.lock();
        perf_info.tasks_spawned += 1;
    }

    pub fn run(&self) {
        loop {
            let next_task = {
                let _locked = irq_lock();
                let i = self.run_queue.lock().pop_front();
                i
            };
            if let Some(task_id) = next_task {
                let waker = task_waker(task_id);
                let mut context = Context::from_waker(&waker);

                let found_task;
                if let Some(task) = self
                    .tasks
                    .lock()
                    .iter_mut()
                    .find(|t| t.read().id == task_id)
                {
                    found_task = Some(task.clone());
                } else {
                    panic!("no task?");
                }

                if let Some(found_task) = found_task {
                    // Run the task
                    let locked_found_task = found_task.read();
                    let uptime_before = get_uptime_us();
                    let poll_result = locked_found_task.poll(&mut context);
                    let uptime_after = get_uptime_us();
                    drop(locked_found_task);

                    // Update counters
                    let mut locked_found_task = found_task.write();
                    locked_found_task.total_yields += 1;
                    locked_found_task.cpu_time_us += uptime_after - uptime_before;
                    let mut perf_info = PERF_INFO.lock();
                    perf_info.total_yields += 1;
                    perf_info.cpu_time_us += uptime_after - uptime_before;
                    drop(locked_found_task);

                    match poll_result {
                        Poll::Ready(()) => {
                            // task done
                            perf_info.tasks_killed += 1;

                            let mut tasks = self.tasks.lock();
                            let idx = tasks
                                .iter()
                                .position(|t| t.read().id == task_id)
                                .expect("Tried to kill non-existent process");
                            println!(
                                "[DBUG] Killing task \"{}\"",
                                AsciiStr(tasks[idx].read().name)
                            );
                            tasks.remove(idx);
                        }
                        Poll::Pending => {
                            // task still needs to run
                        }
                    }
                } else {
                    panic!("no task? 2");
                }
            } else {
                let _locked = irq_lock();
                if self.run_queue.lock().len() != 0 {
                    // Run queue was updated! continue...
                    continue;
                }

                // TODO: replace with `yield_thread`
                // unsafe { asm!("wfi") };
            }
        }
    }

    pub fn proc_list(&self) -> Vec<TaskPerfInfo> {
        let uptime = get_uptime_us();
        self.tasks
            .lock()
            .iter()
            .map(|t| {
                let t = t.read();
                TaskPerfInfo {
                    id: t.id,
                    name: t.name,
                    uptime_us: uptime - t.start_time_us,
                    cpu_time_us: t.cpu_time_us,
                    total_yields: t.total_yields,
                }
            })
            .collect()
    }

    pub fn wake(&self, pid: usize) {
        let _locked = irq_lock();
        let mut run_queue = self.run_queue.lock();
        if !run_queue.contains(&pid) {
            run_queue.push_back(pid);
        }
    }
}

fn task_raw_waker(task_id: usize) -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(task_id: *const ()) -> RawWaker {
        task_raw_waker(task_id as usize)
    }
    fn wake(task_id: *const ()) {
        let _locked = irq_lock();
        EXECUTOR.wait().run_queue.lock().push_back(task_id as usize);
    }
    fn wake_by_ref(task_id: *const ()) {
        let _locked = irq_lock();
        EXECUTOR.wait().run_queue.lock().push_back(task_id as usize);
    }

    let vtable = &RawWakerVTable::new(clone, wake, wake_by_ref, no_op);
    RawWaker::new(task_id as *const (), vtable)
}

pub fn task_waker(task_id: usize) -> Waker {
    unsafe { Waker::from_raw(task_raw_waker(task_id)) }
}

fn thread_raw_waker(thread_id: usize) -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(thread_id: *const ()) -> RawWaker {
        thread_raw_waker(thread_id as usize)
    }
    fn wake(thread_id: *const ()) {
        let _locked = irq_lock();
        threads::EXECUTORS.get().unwrap()[0].wake(thread_id as usize);
    }
    fn wake_by_ref(thread_id: *const ()) {
        let _locked = irq_lock();
        threads::EXECUTORS.get().unwrap()[0].wake(thread_id as usize);
    }

    let vtable = &RawWakerVTable::new(clone, wake, wake_by_ref, no_op);
    RawWaker::new(thread_id as *const (), vtable)
}

pub fn thread_waker(thread_id: usize) -> Waker {
    unsafe { Waker::from_raw(thread_raw_waker(thread_id)) }
}

fn null_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        null_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(null(), vtable)
}

pub fn null_waker() -> Waker {
    unsafe { Waker::from_raw(null_raw_waker()) }
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
}

pub fn run() {
    EXECUTOR.wait().run();
}

pub fn proc_list() -> Vec<TaskPerfInfo> {
    EXECUTOR.wait().proc_list()
}

pub fn wake(pid: usize) {
    EXECUTOR.wait().wake(pid)
}

pub fn perf_report() -> PerfReport {
    PERF_INFO.lock().report()
}

#[macro_export]
macro_rules! spawn_task {
    ($name: expr, $b:block) => {{
        let executor = crate::ktask::EXECUTOR.wait();
        let closure = async move || ($b);
        executor.spawn(crate::ktask::Task::new_raw($name, Box::pin(closure())));
    }};
}
