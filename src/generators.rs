// use std::marker::PhantomData;
// use std::sync::mpsc::{channel, Receiver, Sender};
// use crate::coroutines::{go, yield_coroutine, sleep_write, sleep_read};
//
// pub struct Generator<T> {
//         receiver: Receiver<T>,
//         _phantom: PhantomData<T>,
// }
//
// pub struct Yielder<T> {
//         sender: Sender<T>,
//         _phantom: PhantomData<T>,
// }
//
// impl<T> Generator<T> {
//         pub fn new<F>(f: F) -> Self
//     where
//         F: FnOnce(Yielder<T>) + 'static,
//             T: 'static,
//         {
//         let (sender, receiver) = channel();
//         let yielder = Yielder {
//             sender,
//             _phantom: PhantomData,
//         };
//
//         go(move || {
//             f(yielder);
//         });
//
//         Generator {
//             receiver,
//             _phantom: PhantomData,
//         }
//     }
// }
//
// impl<T> Iterator for Generator<T> {
//         type Item = T;
//
//         fn next(&mut self) -> Option<Self::Item> {
//         self.receiver.recv().ok()
//         }
// }
//
// impl<T> Yielder<T> {
//         pub fn yield_(&self, value: T) {
//         self.sender.send(value).unwrap();
//         yield_coroutine();
//     }
// }
//
// // Example usage
// pub fn range(start: i32, end: i32) -> Generator<i32> {
//     Generator::new(move |y| {
//         let mut current = start;
//         while current < end {
//             y.yield_(current);
//             current += 1;
//         }
//     })
// }
//
// // Helper function for async I/O generators
// pub fn yield_on_read<T>(fd: i32, value: T, yielder: &Yielder<T>) {
//     yielder.yield_(value);
//     sleep_read(fd);
// }
//
// pub fn yield_on_write<T>(fd: i32, value: T, yielder: &Yielder<T>) {
//     yielder.yield_(value);
//     sleep_write(fd);
// }
