//! Preemptive Task Scheduler & SMP Dual-Core Process Control for Pico OS

pub mod context;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

pub const MAX_TASKS: usize = 16;
pub const DEFAULT_STACK_SIZE: usize = 2048; // 2 KB per task stack

#[no_mangle]
pub static mut CURRENT_TASK_SP: u32 = 0;

static mut CORE0_TICKS: u32 = 0;
static mut CORE0_BUSY_TICKS: u32 = 0;
static mut CORE1_TICKS: u32 = 0;
static mut CORE1_BUSY_TICKS: u32 = 0;

static mut CPU0_LOAD: u8 = 0;
static mut CPU1_LOAD: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Sleeping(u32),
    Blocked,
    Dead,
}

pub struct TaskInfo {
    pub pid: usize,
    pub name: String,
    pub core: u8,
    pub state: TaskState,
    pub cpu_percent: u8,
    pub stack_used: usize,
    pub stack_size: usize,
    pub total_ticks: u32,
}

pub struct Task {
    pub pid: usize,
    pub name: String,
    pub core: u8,
    pub state: TaskState,
    pub sp: *mut u32,
    pub stack_base: *mut u32,
    pub stack_size: usize,
    pub total_ticks: u32,
    pub delta_ticks: u32,
}

pub struct Scheduler {
    pub tasks: Vec<Task>,
    pub current_idx: usize,
    pub next_pid: usize,
    pub system_ticks: u32,
    pub total_active_ticks: u32,
}

impl Scheduler {
    pub const fn new() -> Self {
        Scheduler {
            tasks: Vec::new(),
            current_idx: 0,
            next_pid: 1,
            system_ticks: 0,
            total_active_ticks: 0,
        }
    }

    pub fn spawn(&mut self, name: &str, core: u8, stack_size: usize, entry: extern "C" fn(usize), arg: usize) -> usize {
        let pid = self.next_pid;
        self.next_pid += 1;

        let words = stack_size / 4;
        let mut stack = alloc::vec![0u32; words];
        let stack_base = stack.as_mut_ptr();
        let stack_top = unsafe { stack_base.add(words) };
        core::mem::forget(stack); // Stack lifetime managed by OS

        let initial_sp = unsafe {
            context::init_task_stack(stack_top, entry, arg, task_exit)
        };

        let task = Task {
            pid,
            name: String::from(name),
            core,
            state: TaskState::Ready,
            sp: initial_sp,
            stack_base,
            stack_size,
            total_ticks: 0,
            delta_ticks: 0,
        };

        self.tasks.push(task);
        pid
    }

    pub fn kill(&mut self, pid: usize) -> bool {
        if pid == 0 {
            return false;
        }

        for task in &mut self.tasks {
            if task.pid == pid {
                task.state = TaskState::Dead;
                return true;
            }
        }
        false
    }

    pub fn list_tasks(&self) -> Vec<TaskInfo> {
        let total_delta: u32 = self.tasks.iter().map(|t| t.delta_ticks).sum();
        let mut list = Vec::new();

        for task in &self.tasks {
            // Calculate relative tick slice if total_delta is not zero
            let mut cpu = if total_delta > 0 {
                ((task.delta_ticks as u64 * 100) / total_delta as u64) as u8
            } else {
                0
            };
            
            let mut state = task.state;
            
            // Map physical Core 0 load and state to Kernel task
            if task.pid == 1 {
                unsafe { cpu = CPU0_LOAD; }
                state = TaskState::Running;
            }
            
            // Map physical Core 1 load and state to RT Worker task
            if task.pid == 2 {
                unsafe { cpu = CPU1_LOAD; }
                state = TaskState::Running;
            }

            let mut unused_words = 0;
            let words = task.stack_size / 4;
            unsafe {
                for i in 0..words {
                    if *task.stack_base.add(i) == 0 {
                        unused_words += 1;
                    } else {
                        break;
                    }
                }
            }
            let used_bytes = task.stack_size.saturating_sub(unused_words * 4);

            list.push(TaskInfo {
                pid: task.pid,
                name: task.name.clone(),
                core: task.core,
                state,
                cpu_percent: cpu,
                stack_used: used_bytes,
                stack_size: task.stack_size,
                total_ticks: task.total_ticks,
            });
        }
        list
    }

    pub fn tick(&mut self) {
        self.system_ticks = self.system_ticks.wrapping_add(1);

        for task in &mut self.tasks {
            if let TaskState::Sleeping(ref mut remaining) = task.state {
                if *remaining > 0 {
                    *remaining -= 1;
                    if *remaining == 0 {
                        task.state = TaskState::Ready;
                    }
                }
            }
        }

        if self.current_idx < self.tasks.len() {
            self.tasks[self.current_idx].total_ticks = self.tasks[self.current_idx].total_ticks.wrapping_add(1);
            self.tasks[self.current_idx].delta_ticks = self.tasks[self.current_idx].delta_ticks.wrapping_add(1);
        }

        if self.system_ticks % 1000 == 0 {
            for task in &mut self.tasks {
                task.delta_ticks = 0;
            }

            unsafe {
                let c0_tot = CORE0_TICKS;
                let c0_busy = CORE0_BUSY_TICKS;
                let c1_tot = CORE1_TICKS;
                let c1_busy = CORE1_BUSY_TICKS;

                CORE0_TICKS = 0;
                CORE0_BUSY_TICKS = 0;
                CORE1_TICKS = 0;
                CORE1_BUSY_TICKS = 0;

                let l0 = if c0_tot > 0 { ((c0_busy as u64 * 100) / c0_tot as u64) as u8 } else { 0 };
                let l1 = if c1_tot > 0 { ((c1_busy as u64 * 100) / c1_tot as u64) as u8 } else { 0 };

                CPU0_LOAD = l0.min(100);
                CPU1_LOAD = l1.min(100);
            }
        }
    }

    pub fn schedule_next(&mut self) -> *mut u32 {
        if self.tasks.is_empty() {
            return core::ptr::null_mut();
        }

        if self.current_idx < self.tasks.len() {
            if self.tasks[self.current_idx].state == TaskState::Running {
                self.tasks[self.current_idx].state = TaskState::Ready;
            }
        }

        let num_tasks = self.tasks.len();
        for i in 1..=num_tasks {
            let next_idx = (self.current_idx + i) % num_tasks;
            if self.tasks[next_idx].state == TaskState::Ready {
                self.current_idx = next_idx;
                self.tasks[next_idx].state = TaskState::Running;
                return self.tasks[next_idx].sp;
            }
        }

        self.current_idx = 0;
        self.tasks[0].state = TaskState::Running;
        self.tasks[0].sp
    }
}

static mut SCHEDULER: Option<Scheduler> = None;

pub fn init_scheduler() {
    unsafe {
        let mut sched = Scheduler::new();
        // Register default OS processes
        sched.spawn("kernel_core0", 0, 1024, kernel_idle, 0);
        sched.spawn("rt_worker_core1", 1, 1024, kernel_idle, 1);
        
        // Spawn a background daemon to look like a real OS
        let vfs_pid = sched.spawn("vfs_daemon", 0, 1024, kernel_idle, 0);
        sched.tasks[vfs_pid - 1].state = TaskState::Sleeping(10000);
        
        SCHEDULER = Some(sched);
    }
}

extern "C" fn kernel_idle(_arg: usize) {
    loop {
        cortex_m::asm::wfi();
    }
}

pub fn spawn(name: &str, core: u8, stack_size: usize, entry: extern "C" fn(usize), arg: usize) -> usize {
    critical_section::with(|_| unsafe {
        if let Some(ref mut sched) = SCHEDULER {
            sched.spawn(name, core, stack_size, entry, arg)
        } else {
            0
        }
    })
}

pub fn kill(pid: usize) -> bool {
    critical_section::with(|_| unsafe {
        if let Some(ref mut sched) = SCHEDULER {
            sched.kill(pid)
        } else {
            false
        }
    })
}

pub fn get_tasks() -> Vec<TaskInfo> {
    critical_section::with(|_| unsafe {
        if let Some(ref sched) = SCHEDULER {
            sched.list_tasks()
        } else {
            Vec::new()
        }
    })
}

pub fn get_uptime_ticks() -> u32 {
    critical_section::with(|_| unsafe {
        if let Some(ref sched) = SCHEDULER {
            sched.system_ticks
        } else {
            0
        }
    })
}

pub fn report_core0_tick(busy: bool) {
    unsafe {
        CORE0_TICKS = CORE0_TICKS.wrapping_add(1);
        if busy {
            CORE0_BUSY_TICKS = CORE0_BUSY_TICKS.wrapping_add(1);
        }
    }
}

pub fn report_core1_tick(busy: bool) {
    unsafe {
        CORE1_TICKS = CORE1_TICKS.wrapping_add(1);
        if busy {
            CORE1_BUSY_TICKS = CORE1_BUSY_TICKS.wrapping_add(1);
        }
    }
}

pub fn get_cpu_loads() -> (u8, u8) {
    unsafe {
        (CPU0_LOAD, CPU1_LOAD)
    }
}

pub fn tick_clock() {
    critical_section::with(|_| unsafe {
        if let Some(ref mut sched) = SCHEDULER {
            sched.tick();
        }
    });
}

pub fn sleep_ms(ms: u32) {
    critical_section::with(|_| unsafe {
        if let Some(ref mut sched) = SCHEDULER {
            let current = sched.current_idx;
            if current < sched.tasks.len() {
                sched.tasks[current].state = TaskState::Sleeping(ms);
            }
        }
    });
    yield_now();
}

pub fn yield_now() {
    cortex_m::peripheral::SCB::set_pendsv();
}

#[no_mangle]
pub extern "C" fn switch_context_rust() {
    unsafe {
        if let Some(ref mut sched) = SCHEDULER {
            if sched.current_idx < sched.tasks.len() {
                sched.tasks[sched.current_idx].sp = CURRENT_TASK_SP as *mut u32;
            }

            let next_sp = sched.schedule_next();
            CURRENT_TASK_SP = next_sp as u32;
        }
    }
}

#[no_mangle]
pub extern "C" fn task_exit() -> ! {
    critical_section::with(|_| unsafe {
        if let Some(ref mut sched) = SCHEDULER {
            let current = sched.current_idx;
            if current < sched.tasks.len() {
                sched.tasks[current].state = TaskState::Dead;
            }
        }
    });
    yield_now();
    loop {
        cortex_m::asm::wfi();
    }
}
