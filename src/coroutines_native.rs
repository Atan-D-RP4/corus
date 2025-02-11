use std::{
    os::{fd::BorrowedFd, raw::c_void},
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};

use nix::poll::{poll, PollFd, PollFlags, PollTimeout};

const STACK_SIZE: usize = 1024 * 8;

// Extern functions we need to interact with
extern "C" {
    fn _yield_coroutine();
    fn _sleep_read(fd: i32);
    fn _sleep_write(fd: i32);
    fn _restore_coroutine(rsp: *mut c_void);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub enum SleepMode {
    None = 0,
    Read,
    Write,
}

#[derive(Debug)]
pub struct Coroutine {
    pub rsp: usize,
    pub stack_base: usize,
    pub f_ref: Option<extern "C" fn(*mut c_void)>,
}

impl Coroutine {
    fn new() -> Self {
        Self {
            rsp: 0,
            stack_base: 0,
            f_ref: None,
        }
    }
}

struct CoroutineScheduler {
    current: AtomicUsize,
    active: Vec<usize>,
    dead: Vec<usize>,
    coroutines: Vec<Coroutine>,
    asleep: Vec<usize>,
    polls: Vec<PollFd<'static>>,
}

impl CoroutineScheduler {
    fn new() -> Self {
        let mut scheduler = Self {
            current: AtomicUsize::new(0),
            active: Vec::new(),
            dead: Vec::new(),
            coroutines: Vec::new(),
            asleep: Vec::new(),
            polls: Vec::new(),
        };

        // Initialize with main coroutine
        scheduler.coroutines.push(Coroutine::new());
        scheduler.active.push(0);

        scheduler
    }

    fn spawn(&mut self, f: extern "C" fn(*mut c_void), arg: *mut c_void) {
        let id = if let Some(id) = self.dead.pop() {
            id
        } else {
            self.coroutines.push(Coroutine {
                rsp: 0,
                stack_base: 0,
                f_ref: Some(f),
            });
            self.coroutines.len() - 1
        };

        // Allocate and set up the stack
        unsafe {
            let stack = std::alloc::alloc(
                std::alloc::Layout::from_size_align(STACK_SIZE, 16)
                    .expect("Failed to create stack layout"),
            );
            self.coroutines[id].stack_base = stack as usize;

            let mut rsp = (self.coroutines[id].stack_base + STACK_SIZE) as *mut *mut c_void;

            // Set up the stack frame
            rsp = rsp.offset(-1);
            *rsp = _finish_current as *mut c_void;

            rsp = rsp.offset(-1);
            *rsp = f as *mut c_void;

            rsp = rsp.offset(-1);
            *rsp = arg; // rdi

            // Save registers (rbx, rbp, r12-r15)
            for _ in 0..6 {
                rsp = rsp.offset(-1);
                *rsp = ptr::null_mut();
            }

            self.coroutines[id].rsp = rsp as usize;
        }

        self.active.push(id);
    }

    fn switch_context(&mut self, rsp: *mut c_void, mode: SleepMode, fd: i32) {
        let current_id = self.current.load(Ordering::Relaxed);
        self.coroutines[self.active[current_id]].rsp = rsp as usize;

        match mode {
            SleepMode::None => {
                self.current.fetch_add(1, Ordering::Relaxed);
            }
            SleepMode::Read | SleepMode::Write => {
                let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };
                let flags = if mode == SleepMode::Read {
                    PollFlags::POLLRDNORM
                } else {
                    PollFlags::POLLWRNORM
                };

                self.asleep.push(self.active[current_id]);
                self.polls.push(PollFd::new(borrowed_fd, flags));
                self.active.swap_remove(current_id);
            }
        }

        self.process_polls();
        self.schedule_next();
    }

    fn process_polls(&mut self) {
        if self.polls.is_empty() {
            return;
        }

        let timeout = if self.active.is_empty() {
            PollTimeout::NONE
        } else {
            PollTimeout::ZERO
        };

        let _ = poll(&mut self.polls, timeout);

        let mut i = 0;
        while i < self.polls.len() {
            if self.polls[i].revents().unwrap_or(PollFlags::empty()).bits() != 0 {
                let id = self.asleep[i];
                self.polls.swap_remove(i);
                self.asleep.swap_remove(i);
                self.active.push(id);
            } else {
                i += 1;
            }
        }
    }

    fn schedule_next(&mut self) {
        if self.active.is_empty() {
            panic!("No active coroutines");
        }

        let current = self.current.load(Ordering::Relaxed) % self.active.len();
        self.current.store(current, Ordering::Relaxed);

        unsafe {
            _restore_coroutine(self.coroutines[self.active[current]].rsp as *mut c_void);
        }
    }

    fn finish_current(&mut self) {
        let current = self.current.load(Ordering::Relaxed);
        let id = self.active[current];

        if id == 0 {
            panic!("Main coroutine cannot be finished");
        }

        self.dead.push(id);
        self.active.swap_remove(current);

        self.process_polls();
        self.schedule_next();
    }

    fn wake_up(&mut self, target_id: usize) {
        if let Some(pos) = self.asleep.iter().position(|&id| id == target_id) {
            self.asleep.swap_remove(pos);
            self.polls.swap_remove(pos);
            self.active.push(target_id);
        }
    }
}

// Global scheduler instance
static mut SCHEDULER: Option<CoroutineScheduler> = None;

fn get_scheduler() -> &'static mut CoroutineScheduler {
    unsafe {
        if SCHEDULER.is_none() {
            SCHEDULER = Some(CoroutineScheduler::new());
        }
        SCHEDULER.as_mut().unwrap()
    }
}

// Public API
pub fn spawn<F>(f: F)
where
    F: FnOnce(),
{
    extern "C" fn wrapper<F: FnOnce()>(arg: *mut c_void) {
        let boxed_fn = unsafe { Box::from_raw(arg as *mut F) };
        boxed_fn();
    }

    let boxed_fn = Box::new(f);
    let ptr = Box::into_raw(boxed_fn) as *mut c_void;
    get_scheduler().spawn(wrapper::<F>, ptr);
}

#[no_mangle]
unsafe fn _switch_context(rsp: *mut c_void, mode: SleepMode, fd: i32) {
    get_scheduler().switch_context(rsp, mode, fd);
}

#[no_mangle]
unsafe fn _finish_current() {
    get_scheduler().finish_current();
}

pub fn yield_coroutine() {
    unsafe { _yield_coroutine() }
}

pub fn sleep_read(fd: i32) {
    unsafe { _sleep_read(fd) }
}

pub fn sleep_write(fd: i32) {
    unsafe { _sleep_write(fd) }
}

pub fn id() -> usize {
    let scheduler = get_scheduler();
    let current = scheduler.current.load(Ordering::Relaxed);
    scheduler.active[current]
}

pub fn alive() -> usize {
    get_scheduler().active.len()
}

pub fn wake_up(id: usize) {
    get_scheduler().wake_up(id);
}

pub fn handle(id: usize) -> &'static Coroutine {
    &get_scheduler().coroutines[id]
}
