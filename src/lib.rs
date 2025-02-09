#![feature(naked_functions)]
// src/lib.rs
use std::arch::naked_asm;
use std::os::raw::c_void;

unsafe extern "C" {
    pub unsafe fn _go(f: extern "C" fn(*mut c_void), arg: *mut c_void);
    pub unsafe fn _id() -> usize;
    pub unsafe fn _alive() -> usize;
    pub unsafe fn _wake_up(id: usize);
}

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

pub fn id() -> usize {
    unsafe { _id() }
}

pub fn alive() -> usize {
    unsafe { _alive() }
}

pub fn wake_up(id: usize) {
    unsafe {_wake_up(id) }
}

#[naked]
#[no_mangle]
pub unsafe fn yield_coroutine() {
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

// #[repr(C)]
// enum Sleep_Mode {
//     SmNone = 0,
//     SmRead,
//     SmWrite,
// }
//
// #[derive(Debug)]
// struct Coroutine {
//     rsp: usize,
//     stack_base: usize,
// }
//
// static STACK_SIZE: usize = 1024 * 8;
//
// use nix::poll::PollFd;
// use std::sync::{Mutex, LazyLock};
//
// static CURRENT: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
// static ALIVE: LazyLock<Mutex<Vec<usize>>> = LazyLock::new(|| Mutex::new(vec![0]));
// static DEAD: LazyLock<Mutex<Vec<usize>>> = LazyLock::new(|| Mutex::new(Vec::new()));
// static COROUTINES: LazyLock<Mutex<Vec<Coroutine>>> = LazyLock::new(|| {
//     Mutex::new(vec![Coroutine {
//         rsp: 0,
//         stack_base: 0,
//     }])
// });
// static ASLEEP: LazyLock<Mutex<Vec<usize>>> = LazyLock::new(|| Mutex::new(Vec::new()));
// static POLLS: LazyLock<Mutex<Vec<PollFd>>> = LazyLock::new(|| Mutex::new(Vec::new()));
//
// #[no_mangle]
// #[inline(never)]
// pub unsafe fn coroutine_switch_context(rsp: *mut c_void, sm: Sleep_Mode, fd: i32) {
//     use nix::poll::{poll, PollFlags, PollTimeout};
//     use std::os::fd::BorrowedFd;
//
//     let mut coroutines = COROUTINES.lock().unwrap();
//     let mut alive = ALIVE.lock().unwrap();
//     let mut current = *CURRENT.lock().unwrap();
//     let mut asleep = ASLEEP.lock().unwrap();
//     let mut polls = POLLS.lock().unwrap();
//
//     let rsp = rsp as usize;
//     println!("Switching from coroutine {}", current);
//     println!("Alive: {:?}", alive);
//     println!("Switching to coroutine {}", alive[current]);
//     println!("Coroutines: {:?}", coroutines);
//     coroutines[alive[current]].rsp = rsp;
//
//     match sm {
//         Sleep_Mode::SmNone => {
//             current = (current + 1) % alive.len();
//         }
//         Sleep_Mode::SmRead => {
//             let fd = unsafe { BorrowedFd::borrow_raw(fd) };
//             let pfd: PollFd = PollFd::new(fd, PollFlags::POLLRDNORM);
//             polls.push(pfd);
//             alive.remove(current);
//         }
//
//         Sleep_Mode::SmWrite => {
//             let fd = unsafe { BorrowedFd::borrow_raw(fd) };
//             let pfd: PollFd = PollFd::new(fd, PollFlags::POLLRDNORM);
//             polls.push(pfd);
//             alive.remove(current);
//         }
//     }
//
//     if polls.len() > 0 {
//         let mut pollfds = polls.as_mut_slice();
//         let timeout = if alive.len() == 0 {
//             PollTimeout::NONE
//         } else {
//             PollTimeout::ZERO
//         };
//         let _ = poll(&mut pollfds, timeout);
//         let mut i = 0;
//         while i < pollfds.len() {
//             if pollfds[i].revents().is_some() {
//                 let mut id = 0;
//                 for j in 0..alive.len() {
//                     if alive[j] == i {
//                         id = j;
//                         break;
//                     }
//                 }
//                 asleep.push(id);
//                 alive.remove(i);
//             }
//             i += 1;
//         }
//     }
//
//     if alive.len() > 0 {
//         current %= alive.len();
//         coroutine_restore_context(rsp as *mut c_void);
//         println!("Switching to coroutine {}", current);
//         println!("coroutine_switch_context");
//         return;
//     }
//     println!("No more alive coroutines");
// }
//
// #[no_mangle]
// pub unsafe fn finish_current() {
//     use nix::poll::{poll, PollFlags, PollTimeout};
//     println!("Finish current");
//
//     let coroutines = COROUTINES.lock().unwrap();
//     let mut alive = ALIVE.lock().unwrap();
//     let mut current = *CURRENT.lock().unwrap();
//     let mut dead = DEAD.lock().unwrap();
//     let mut polls = POLLS.lock().unwrap();
//     let mut asleep = ASLEEP.lock().unwrap();
//
//     if alive[current] == 0 {
//         return;
//     }
//
//     dead.push(alive[current]);
//     alive.remove(current);
//
//     if polls.len() > 0 {
//         let n = polls.len();
//         let mut pollfds = polls.clone();
//         let mut pollfds = pollfds.as_mut_slice();
//         let timeout = if alive.len() == 0 {
//             PollTimeout::NONE
//         } else {
//             PollTimeout::ZERO
//         };
//         let _ = poll(&mut pollfds, timeout);
//         let mut i = 0;
//         while i < n {
//             if pollfds[i].revents().unwrap_or(PollFlags::empty()).bits() != 0 {
//                 let id = asleep[i];
//                 asleep.remove(i);
//                 polls.remove(i);
//                 alive.push(id);
//             } else {
//                 i += 1;
//             }
//         }
//     }
//
//     if alive.len() > 0 {
//         current %= alive.len();
//         *CURRENT.lock().unwrap() = current;
//         coroutine_restore_context(coroutines[alive[current]].rsp as *mut c_void);
//     }
// }
//
// pub unsafe fn _go(f: extern "C" fn(*mut c_void), arg: *mut c_void) {
//     use libc::{
//         mmap, MAP_ANONYMOUS, MAP_FAILED, MAP_GROWSDOWN, MAP_PRIVATE, MAP_STACK, PROT_READ,
//         PROT_WRITE,
//     };
//     let mut coroutines = COROUTINES.lock().unwrap();
//     let mut dead = DEAD.lock().unwrap();
//
//     // Get id from dead coroutines or create new one
//     let id = if dead.len() > 0 {
//         dead.pop().unwrap()
//     } else {
//         coroutines.push(Coroutine {
//             rsp: 0,
//             stack_base: 0,
//         });
//         let id = coroutines.len() - 1;
//         let stack_base = unsafe {
//             mmap(
//                 std::ptr::null_mut(),
//                 STACK_SIZE,
//                 PROT_WRITE | PROT_READ,
//                 MAP_PRIVATE | MAP_STACK | MAP_ANONYMOUS | MAP_GROWSDOWN,
//                 -1,
//                 0,
//             )
//         };
//         if stack_base == MAP_FAILED {
//             panic!("Failed to create new Stack")
//         }
//         coroutines[id].stack_base = stack_base as usize;
//         id
//     };
//
//     // Create the trampoline function that will call our closure
//     extern "C" fn trampoline<F: FnOnce()>(f: *mut c_void) {
//         let f = unsafe { Box::from_raw(f as *mut F) };
//         f();
//     }
//
//     // Box the closure and get a raw pointer to it
//     let boxed_f = Box::new(f);
//     let f_ptr = Box::into_raw(boxed_f);
//
//     let mut rsp = (coroutines[id].stack_base + STACK_SIZE) as *mut *mut c_void;
//     println!("rsp: {}", rsp as usize);
//
//     unsafe {
//         // Push finish_current function pointer
//         rsp = rsp.offset(-1);
//         *rsp = finish_current as *mut c_void;
//
//         // Push trampoline function pointer
//         rsp = rsp.offset(-1);
//         *rsp = f_ptr as *mut c_void;
//
//         // Push closure pointer as argument
//         rsp = rsp.offset(-1);
//         *rsp = f_ptr as *mut c_void;
//
//         // Push callee-saved registers
//         rsp = rsp.offset(-1);
//         *rsp = arg as *mut c_void;   // rbx
//
//         rsp = rsp.offset(-1);
//         *rsp = std::ptr::null_mut(); // rbp
//         rsp = rsp.offset(-1);
//         *rsp = std::ptr::null_mut(); // r12
//         rsp = rsp.offset(-1);
//         *rsp = std::ptr::null_mut(); // r13
//         rsp = rsp.offset(-1);
//         *rsp = std::ptr::null_mut(); // r14
//         rsp = rsp.offset(-1);
//         *rsp = std::ptr::null_mut(); // r15
//     }
//
//     coroutines[id].rsp = rsp as usize;
//     ALIVE.lock().unwrap().push(id);
// }
//
// pub unsafe fn _id() -> usize {
//     ALIVE.lock().unwrap()[*CURRENT.lock().unwrap()]
// }
// pub unsafe fn _alive() -> usize {
//     ALIVE.lock().unwrap().len()
// }
// pub unsafe fn _wake_up(id: usize) {
//     let mut asleep = ASLEEP.lock().unwrap();
//     let mut alive = ALIVE.lock().unwrap();
//     let mut polls = POLLS.lock().unwrap();
//     if let Some(pos) = asleep.iter().position(|&x| x == id) {
//         asleep.remove(pos);
//         polls.remove(pos);
//         alive.push(id);
//     }
// }
