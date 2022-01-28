use crate::arch::aarch64::exceptions::ExceptionContext;
use crate::arch::aarch64::mmio::get_uptime_us;
use crate::arch::aarch64::mmu::PageTable;
use crate::arch::aarch64::phymem;
use crate::ktask;
use crate::prelude::*;
use core::pin::Pin;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::{Mutex, Once, RwLock};

pub(crate) const THREAD_TIMEOUT_US: u64 = 10_000;
pub(crate) const CORE_COUNT: usize = 1;
pub(crate) static EXECUTORS: Once<[SimpleThreadExecutor; CORE_COUNT]> = Once::new();
// static PERF_INFO: Mutex<PerfInfo> = Mutex::new(PerfInfo::new());
static PID_COUNTER: AtomicUsize = AtomicUsize::new(1);

pub struct PerfInfo {
    pub cpu_time_us: u64,
    pub total_yields: u64,
    pub threads_spawned: u64,
    pub threads_killed: u64,
}

pub struct ThreadPerfInfo {
    pub id: usize,
    pub name: &'static [u8],
    pub uptime_us: u64,
    pub cpu_time_us: u64,
    pub total_yields: u64,
}

pub unsafe fn current_core() -> usize {
    get_msr!(mpidr_el1) as usize & 0x3
}

#[derive(Debug)]
pub struct PerfReport {
    uptime_us: u64,
    cpu_time_us: u64,
    total_yields: u64,
    avg_time_between_yields_us: u64,
    threads_spawned: u64,
    threads_killed: u64,
    current_threads: u64,
}

pub struct Thread {
    id: usize,
    name: &'static [u8],
    start_time_us: u64,
    last_enter_time_us: u64,
    cpu_time_us: u64,
    total_yields: u64,
    state: ExceptionContext,
    page_tables: Option<Pin<Box<PageTable>>>,
    // kernel_stack: Pin<Box<PageAligned<{ 4096 * 32 }>>>,
    kernel_stack: &'static [u8],
}

impl Thread {
    pub fn new(
        name: &'static [u8],
        state: ExceptionContext,
        page_tables: Option<Pin<Box<PageTable>>>,
    ) -> Thread {
        let id = PID_COUNTER.fetch_add(1, Ordering::SeqCst);
        println!("Creating thread #{}", id);
        let stack = unsafe {
            let mut phymem = phymem::PHYMEM_FREE_LIST.lock();
            let kernel_virtmem = phymem
                .alloc_pages(64)
                // 256KiB
                .expect("Failed to allocate thread stack");
            kernel_virtmem.virt()

            // Box::pin(PageAligned([0u8; 4096 * 32])) // This didn't work for some reason
        };

        let mut thread = Thread {
            id,
            name,
            start_time_us: get_uptime_us(),
            last_enter_time_us: 0,
            cpu_time_us: 0,
            total_yields: 0,
            state,
            page_tables,
            kernel_stack: stack, // TODO: Use this
        };

        println!(
            "Allocated kernel stack for \"{}\" at {:p}",
            AsciiStr(name),
            thread.kernel_stack.as_ptr()
        );

        if thread.state.sp == 0 {
            thread.state.sp =
                (thread.kernel_stack.as_ptr() as usize + thread.kernel_stack.len()) as u64;
        }

        thread
    }

    pub fn kill(&self) {
        println!("Killing thread #{}", self.id);
        for executor in EXECUTORS.get().unwrap() {
            executor.unregister_thread(self.id);
        }
        println!("Killed!");
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

pub struct SimpleThreadExecutor {
    threads: Mutex<Vec<Arc<RwLock<Thread>>>>,
    run_queue: Mutex<VecDeque<usize>>,
    last_switch: AtomicU64,
    current_thread: Mutex<Option<Arc<RwLock<Thread>>>>,
}

impl SimpleThreadExecutor {
    pub fn new() -> SimpleThreadExecutor {
        SimpleThreadExecutor {
            threads: Mutex::new(Vec::new()),
            run_queue: Mutex::new(VecDeque::new()),
            last_switch: AtomicU64::new(0),
            current_thread: Mutex::new(None),
        }
    }

    pub fn spawn(&self, thread: Thread) {
        let id = thread.id;
        self.threads.lock().push(Arc::new(RwLock::new(thread)));

        {
            let _locked = irq_lock();
            self.run_queue.lock().push_back(id);
        }

        // let mut perf_info = PERF_INFO.lock();
        // perf_info.threads_spawned += 1;
    }

    pub fn did_timeout(&self) -> bool {
        let last_switch = self.last_switch.load(Ordering::SeqCst);
        let current_uptime = get_uptime_us();

        let _locked = irq_lock();
        if current_uptime - THREAD_TIMEOUT_US > last_switch && self.run_queue.lock().len() != 0 {
            self.last_switch
                .store(last_switch + THREAD_TIMEOUT_US, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    pub fn switch(&self, current_state: &mut ExceptionContext) -> usize {
        loop {
            let next_tid = {
                let _locked = irq_lock();
                let i = self.run_queue.lock().pop_front();
                i
            };
            if let Some(thread_id) = next_tid {
                let next_thread = self
                    .threads
                    .lock()
                    .iter_mut()
                    .find(|t| t.read().id == thread_id)
                    .map(|t| t.clone())
                    .expect("no thread?");

                let next_state = {
                    let mut next_thread = next_thread.write();

                    // Switch to next page tables
                    unsafe {
                        if let Some(page_tables) = &next_thread.page_tables {
                            set_msr!(ttbr0_el1, (page_tables.0.as_ptr() as u64) & 0x7FFFFFFFFF)
                        } else {
                            set_msr!(ttbr0_el1, 0);
                        }
                    };

                    next_thread.last_enter_time_us = get_uptime_us();

                    // Get context to switch to
                    next_thread.state
                };

                let last_thread = self.current_thread.lock().replace(next_thread);
                if let Some(last_thread) = &last_thread {
                    let mut last_thread = last_thread.write();
                    last_thread.state = *current_state;
                    last_thread.total_yields += 1;
                    last_thread.cpu_time_us += get_uptime_us() - last_thread.last_enter_time_us;
                }

                *current_state = next_state;
                break last_thread.map(|t| t.read().id).unwrap_or(0);
            } else {
                let _locked = irq_lock();
                if self.run_queue.lock().len() != 0 {
                    // Run queue was updated! continue...
                    continue;
                }
                // unsafe { asm!("wfi") };
            }
        }
    }

    pub fn wake(&self, thread_id: usize) {
        self.run_queue.lock().push_back(thread_id)
    }

    pub fn current_thread(&self) -> Option<Arc<RwLock<Thread>>> {
        self.current_thread.lock().clone()
    }

    pub fn unregister_thread(&self, tid: usize) {
        self.run_queue.lock().retain(|t| *t != tid);
        self.threads.lock().retain(|t| t.read().id != tid);

        let mut current_thread = self.current_thread.lock();
        let is_current = if let Some(current_thread) = &*current_thread {
            current_thread.read().id == tid
        } else {
            false
        };
        if is_current {
            *current_thread = None;
        }
    }

    pub fn proc_list(&self) -> Vec<ThreadPerfInfo> {
        let uptime = get_uptime_us();
        self.threads
            .lock()
            .iter()
            .map(|t| {
                let t = t.read();
                ThreadPerfInfo {
                    id: t.id,
                    name: t.name,
                    uptime_us: uptime - t.start_time_us,
                    cpu_time_us: t.cpu_time_us,
                    total_yields: t.total_yields,
                }
            })
            .collect()
    }
}

pub(crate) unsafe fn init() {
    EXECUTORS.call_once(|| {
        let executor = SimpleThreadExecutor::new();

        executor.spawn(Thread::new(
            b"ktask/0",
            ExceptionContext {
                gpr: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0,
                ],
                lr: 0,
                pc: ktask_thread as unsafe extern "C" fn() as *const () as u64,
                sp: 0,
                spsr: 0x304,
            },
            None,
        ));

        executor.spawn(Thread::new(
            b"ktask/1",
            ExceptionContext {
                gpr: [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0,
                ],
                lr: 0,
                pc: ktask_thread as unsafe extern "C" fn() as *const () as u64,
                sp: 0,
                spsr: 0x304,
            },
            None,
        ));

        [executor]
    });
}

pub unsafe fn yield_thread() {}

pub fn proc_list() -> Vec<ThreadPerfInfo> {
    EXECUTORS.wait()[0].proc_list()
}

unsafe extern "C" fn ktask_thread() {
    ktask::run();
}
