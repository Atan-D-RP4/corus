#![feature(naked_functions)]
use std::{
    arch::naked_asm,
    os::{fd::BorrowedFd, raw::c_void},
    ptr,
};

use nix::poll::{poll, PollFd, PollFlags, PollTimeout};

// Example safe wrapper
pub fn go<F>(f: F)
where
    F: FnOnce(),
{
    extern "C" fn wrapper<F: FnOnce()>(arg: *mut c_void) {
        let boxed_fn = unsafe { Box::from_raw(arg as *mut F) };
        boxed_fn();
    }

    let boxed_fn = Box::new(f);
    let ptr = Box::into_raw(boxed_fn) as *mut c_void;
    unsafe { _go(wrapper::<F>, ptr) };
}

pub fn yield_coroutine() {
    unsafe { _yield_coroutine() }
}

pub fn id() -> usize {
    unsafe { _id() }
}

pub fn alive() -> usize {
    unsafe { _alive() }
}

pub fn wake_up(id: usize) {
    unsafe { _wake_up(id) }
}

#[naked]
#[no_mangle]
pub unsafe fn _yield_coroutine() {
    naked_asm!(
        "push rdi",
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov rdi, rsp", // rsp
        "mov rsi, 0",   // sm = SM_NONE
        "jmp coroutine_switch_context"
    );
}

#[naked]
#[no_mangle]
pub unsafe fn sleep_read(fd: i32) {
    naked_asm!(
        "push rdi",
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov rdx, rdi", // fd
        "mov rdi, rsp", // rsp
        "mov rsi, 1",   // sm = SM_READ
        "jmp coroutine_switch_context"
    );
}

#[naked]
#[no_mangle]
pub unsafe fn sleep_write(fd: i32) {
    naked_asm!(
        "push rdi",
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov rdx, rdi", // fd
        "mov rdi, rsp", // rsp
        "mov rsi, 2",   // sm = SM_WRITE
        "jmp coroutine_switch_context"
    );
}

#[naked]
#[no_mangle]
pub unsafe fn coroutine_restore_context(rsp: *mut c_void) {
    naked_asm!(
        "mov rsp, rdi",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "pop rdi",
        "ret",
    );
}

static mut CURRENT: usize = 0;
static mut ACTIVE: Vec<usize> = Vec::new();
static mut DEAD: Vec<usize> = Vec::new();
static mut COROUTINES: Vec<Context> = Vec::new();
static mut ASLEEP: Vec<usize> = Vec::new();
static mut POLLS: Vec<PollFd> = Vec::new();

const STACK_SIZE: usize = 1024 * 8;

#[derive(Debug)]
struct Context {
    rsp: usize,
    stack_base: usize,
}

#[repr(C)]
#[derive(PartialEq)]
pub enum SleepMode {
    SmNone = 0,
    SmRead,
    SmWrite,
}

#[no_mangle]
#[inline(never)]
pub unsafe fn coroutine_switch_context(rsp: *mut c_void, sm: SleepMode, fd: i32) {
    COROUTINES[ACTIVE[CURRENT]].rsp = rsp as usize;

    let borrowed_fd = BorrowedFd::borrow_raw(fd);
    match sm {
        SleepMode::SmNone => {
            CURRENT += 1;
        }
        SleepMode::SmRead => {
            ASLEEP.push(ACTIVE[CURRENT]);
            let borrowed_fd = BorrowedFd::borrow_raw(fd);
            POLLS.push(PollFd::new(borrowed_fd, PollFlags::POLLRDNORM));
            ACTIVE.swap_remove(CURRENT);
        }
        SleepMode::SmWrite => {
            ASLEEP.push(ACTIVE[CURRENT]);
            POLLS.push(PollFd::new(borrowed_fd, PollFlags::POLLWRNORM));
            ACTIVE.swap_remove(CURRENT);
        }
    }

    if !POLLS.is_empty() {
        let timeout = if ACTIVE.is_empty() {
            PollTimeout::NONE
        } else {
            PollTimeout::ZERO
        };
        let _ = poll(&mut POLLS, timeout);

        let mut i = 0;
        while i < POLLS.len() {
            if POLLS[i].revents().unwrap_or(PollFlags::empty()).bits() != 0 {
                let id = ASLEEP[i];
                POLLS.swap_remove(i);
                ASLEEP.swap_remove(i);
                ACTIVE.push(id);
            } else {
                i += 1;
            }
        }
    }

    if ACTIVE.is_empty() {
        panic!("No active coroutines");
    }
    CURRENT %= ACTIVE.len();
    coroutine_restore_context(COROUTINES[ACTIVE[CURRENT]].rsp as *mut c_void);
}

#[no_mangle]
pub unsafe fn finish_current() {
    if ACTIVE[CURRENT] == 0 {
        panic!("Main Coroutine with id == 0 should never reach this place");
    }

    DEAD.push(ACTIVE[CURRENT]);
    ACTIVE.swap_remove(CURRENT);

    if !POLLS.is_empty() {
        let timeout = if ACTIVE.is_empty() {
            PollTimeout::NONE
        } else {
            PollTimeout::ZERO
        };
        let _ = poll(&mut POLLS, timeout);

        let mut i = 0;
        while i < POLLS.len() {
            if POLLS[i].revents().unwrap_or(PollFlags::empty()).bits() != 0 {
                let id = ASLEEP[i];
                POLLS.swap_remove(i);
                ASLEEP.swap_remove(i);
                ACTIVE.push(id);
            } else {
                i += 1;
            }
        }
    }

    if ACTIVE.is_empty() {
        panic!("No active coroutines");
    }
    CURRENT %= ACTIVE.len();
    coroutine_restore_context(COROUTINES[ACTIVE[CURRENT]].rsp as *mut c_void);
}

pub unsafe fn _go(f: extern "C" fn(*mut c_void), arg: *mut c_void) {
    if COROUTINES.is_empty() {
        COROUTINES.push(Context {
            rsp: 0,
            stack_base: 0,
        });
        ACTIVE.push(0);
    }

    let id = if !DEAD.is_empty() {
        DEAD.pop().unwrap()
    } else {
        COROUTINES.push(Context {
            rsp: 0,
            stack_base: 0,
        });
        let id = COROUTINES.len() - 1;

        // Rust idiomatic stack allocation with alignment considered
        let stack = std::alloc::alloc(std::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap());
        COROUTINES[id].stack_base = stack as usize;
        id
    };

    let mut rsp = (COROUTINES[id].stack_base + STACK_SIZE) as *mut *mut c_void;

    rsp = rsp.offset(-1);
    *rsp = finish_current as *mut c_void;

    rsp = rsp.offset(-1);
    *rsp = f as *mut c_void;

    rsp = rsp.offset(-1);
    *rsp = arg; // rdi

    for _ in 0..6 {
        // rbx, rbp, r12-r15
        rsp = rsp.offset(-1);
        *rsp = ptr::null_mut();
    }

    COROUTINES[id].rsp = rsp as usize;
    ACTIVE.push(id);
}

pub unsafe fn _id() -> usize {
    ACTIVE[CURRENT]
}

pub unsafe fn _alive() -> usize {
    ACTIVE.len()
}

pub unsafe fn _wake_up(id: usize) {
    for i in 0..ASLEEP.len() {
        if ASLEEP[i] == id {
            ASLEEP.swap_remove(i);
            POLLS.swap_remove(i);
            ACTIVE.push(id);
            return;
        }
    }
}
