use std::alloc::{alloc,Layout};
// NOTE:
// The following commented out code only works on the nightly compiler
// with naked functions feature enabled
//
// pub mod nakeds {
//     use std::arch::naked_asm;
//     use std::ffi::c_void;
//
//     #[naked]
//     #[no_mangle]
//     pub unsafe fn _yield_coroutine() {
//         naked_asm!(
//             "push rdi",
//             "push rbp",
//             "push rbx",
//             "push r12",
//             "push r13",
//             "push r14",
//             "push r15",
//             "mov rdi, rsp", // rsp
//             "mov rsi, 0",   // sm = SM_NONE
//             "jmp switch_context"
//         );
//     }
//
//     #[naked]
//     #[no_mangle]
//     pub unsafe fn _sleep_read(fd: i32) {
//         naked_asm!(
//             "push rdi",
//             "push rbp",
//             "push rbx",
//             "push r12",
//             "push r13",
//             "push r14",
//             "push r15",
//             "mov rdx, rdi", // fd
//             "mov rdi, rsp", // rsp
//             "mov rsi, 1",   // sm = SM_READ
//             "jmp switch_context"
//         );
//     }
//
//     #[naked]
//     #[no_mangle]
//     pub unsafe fn _sleep_write(fd: i32) {
//         naked_asm!(
//             "push rdi",
//             "push rbp",
//             "push rbx",
//             "push r12",
//             "push r13",
//             "push r14",
//             "push r15",
//             "mov rdx, rdi", // fd
//             "mov rdi, rsp", // rsp
//             "mov rsi, 2",   // sm = SM_WRITE
//             "jmp switch_context"
//         );
//     }
//
//     #[naked]
//     #[no_mangle]
//     pub unsafe fn _restore_coroutine(rsp: *mut c_void) {
//         naked_asm!(
//             "mov rsp, rdi",
//             "pop r15",
//             "pop r14",
//             "pop r13",
//             "pop r12",
//             "pop rbx",
//             "pop rbp",
//             "pop rdi",
//             "ret",
//         );
//     }
// }
// use crate::coroutines::nakeds::{_restore_coroutine, _sleep_read, _sleep_write, _yield_coroutine};

use std::{
    os::{fd::BorrowedFd, raw::c_void},
    ptr,
};

use nix::poll::{poll, PollFd, PollFlags, PollTimeout};

// Compile this when not using the nightly compiler
unsafe extern "C" {
    pub unsafe fn _yield_coroutine();
    pub unsafe fn _sleep_read(fd: i32);
    pub unsafe fn _sleep_write(fd: i32);
    pub unsafe fn _restore_coroutine(rsp: *mut c_void);
}

static mut CURRENT: usize = 0;
static mut ACTIVE: Vec<usize> = Vec::new();
static mut DEAD: Vec<usize> = Vec::new();
static mut COROUTINES: Vec<Coroutine> = Vec::new();
static mut ASLEEP: Vec<usize> = Vec::new();
static mut POLLS: Vec<PollFd> = Vec::new();

const STACK_SIZE: usize = 1024 * 8;

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

#[repr(C)]
#[derive(PartialEq)]
#[allow(dead_code)]
enum SleepMode {
    None = 0,
    Read,
    Write,
}

fn get_state() -> (
    *mut usize,
    *mut Vec<usize>,
    *mut Vec<usize>,
    *mut Vec<Coroutine>,
    *mut Vec<usize>,
    *mut Vec<PollFd<'static>>,
) {
    (
        &raw mut CURRENT,
        &raw mut ACTIVE,
        &raw mut DEAD,
        &raw mut COROUTINES,
        &raw mut ASLEEP,
        &raw mut POLLS,
    )
}

#[no_mangle]
#[inline(never)]
unsafe fn switch_context(rsp: *mut c_void, sm: SleepMode, fd: i32) {
    let (current, active, _, coroutines, asleep, polls) = get_state();
    let coroutines = &mut *coroutines;
    let active = &mut *active;
    let asleep = &mut *asleep;

    let polls = &mut *polls;
    coroutines[active[*current]].rsp = rsp as usize;

    let borrowed_fd = BorrowedFd::borrow_raw(fd);
    match sm {
        SleepMode::None => {
            *current += 1;
        }
        SleepMode::Read => {
            asleep.push(active[*current]);
            let borrowed_fd = BorrowedFd::borrow_raw(fd);
            polls.push(PollFd::new(borrowed_fd, PollFlags::POLLRDNORM));
            active.swap_remove(*current);
        }
        SleepMode::Write => {
            asleep.push(active[*current]);
            polls.push(PollFd::new(borrowed_fd, PollFlags::POLLWRNORM));
            active.swap_remove(*current);
        }
    }

    if !polls.is_empty() {
        let timeout = if active.is_empty() {
            PollTimeout::NONE
        } else {
            PollTimeout::ZERO
        };
        let _ = poll(polls, timeout);

        let mut i = 0;
        while i < polls.len() {
            if polls[i].revents().unwrap_or(PollFlags::empty()).bits() != 0 {
                let id = asleep[i];
                polls.swap_remove(i);
                asleep.swap_remove(i);
                active.push(id);
            } else {
                i += 1;
            }
        }
    }

    if active.is_empty() {
        panic!("no active coroutines");
    }
    *current %= active.len();
    _restore_coroutine(coroutines[active[*current]].rsp as *mut c_void);
}

#[no_mangle]
unsafe fn finish_current() {
    let (current, active, dead, coroutines, asleep, polls) = get_state();
    let coroutines = &mut *coroutines;
    let active = &mut *active;
    let active = &mut *active;
    let asleep = &mut *asleep;
    let dead = &mut *dead;
    let mut polls = &mut *polls;
    if active[*current] == 0 {
        panic!("Main Coroutine with id == 0 should never reach this place");
    }

    dead.push(active[*current]);
    active.swap_remove(*current);

    if !polls.is_empty() {
        let timeout = if active.is_empty() {
            PollTimeout::NONE
        } else {
            PollTimeout::ZERO
        };
        let _ = poll(&mut polls, timeout);

        let mut i = 0;
        while i < polls.len() {
            if polls[i].revents().unwrap_or(PollFlags::empty()).bits() != 0 {
                let id = asleep[i];
                polls.swap_remove(i);
                asleep.swap_remove(i);
                active.push(id);
            } else {
                i += 1;
            }
        }
    }

    if active.is_empty() {
        panic!("no active coroutines");
    }
    *current %= active.len();
    _restore_coroutine(coroutines[active[*current]].rsp as *mut c_void);
}

unsafe fn _spawn(f: extern "C" fn(*mut c_void), arg: *mut c_void) {
    let (_, active, dead, coroutines, _, _) = get_state();
    let coroutines = &mut *coroutines;
    let active = &mut *active;
    let active = &mut *active;
    let dead = &mut *dead;
    if coroutines.is_empty() {
        coroutines.push(Coroutine::new());
        active.push(0);
    }

    let id = if !dead.is_empty() {
        dead.pop().unwrap()
    } else {
        coroutines.push(Coroutine {
            rsp: 0,
            stack_base: 0,
            f_ref: Some(f),
        });
        let id = coroutines.len() - 1;

        // Rust idiomatic stack allocation with alignment considered
        let stack = alloc(Layout::from_size_align(STACK_SIZE, 16).unwrap());
        coroutines[id].stack_base = stack as usize;
        id
    };

    let mut rsp = (coroutines[id].stack_base + STACK_SIZE) as *mut *mut c_void;

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

    coroutines[id].rsp = rsp as usize;
    active.push(id);
}

unsafe fn _id() -> usize {
    let (current, active, _, _, _, _) = get_state();
    let active = &mut *active;
    active[*current]
}

unsafe fn _alive() -> usize {
    let (_, active, _, _, _, _) = get_state();
    let active = &mut *active;
    active.len()
}

unsafe fn _wake_up(id: usize) {
    let (_, active, _, _, asleep, polls) = get_state();
    let active = &mut *active;
    let asleep = &mut *asleep;
    let polls = &mut *polls;
    for i in 0..asleep.len() {
        if asleep[i] == id {
            asleep.swap_remove(i);
            polls.swap_remove(i);
            active.push(id);
            return;
        }
    }
}

// Safe wrappers
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
    unsafe { _spawn(wrapper::<F>, ptr) };
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
    unsafe { _id() }
}

pub fn alive() -> usize {
    unsafe { _alive() }
}

pub fn wake_up(id: usize) {
    unsafe { _wake_up(id) }
}

pub fn handle(id: usize) -> &'static Coroutine {
    let coroutines = unsafe {
        let (_, _, _, coroutines, _, _) = get_state();
        &mut *coroutines
    };
    &coroutines[id]
}
