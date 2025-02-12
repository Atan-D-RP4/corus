use std::ffi::c_void;

#[derive(Debug, Clone, Copy)]
pub struct Generator {
    stack_base: usize,
    rsp: usize,
    fresh: bool,
    dead: bool,
}

extern "C" {
    fn _generator_next(g: *mut c_void, arg: *mut c_void);
    fn _generator_restore_context(rsp: *mut c_void);
    fn _generator_restore_context_with_return(rsp: *mut c_void, arg: *mut c_void);
    fn _generator_yield(arg: *mut c_void);

}

thread_local! {
    static CURRENT: Vec<Generator> = vec![Generator {
        stack_base: 0,
        rsp: 0,
        fresh: true,
        dead: false,
    }];
}

const STACK_SIZE: usize = 1024 * 8;

// void generator_return(void *arg, void *rsp) {
//   da_last(&generator_stack)->rsp = rsp;
//   generator_stack.count -= 1;
//   generator_restore_context_with_return(da_last(&generator_stack)->rsp, arg);
// }
//
// void generator__finish_current(void) {
//   da_last(&generator_stack)->dead = true;
//   generator_stack.count -= 1;
//   generator_restore_context_with_return(da_last(&generator_stack)->rsp, NULL);
// }
//
// Generator *generator_create(void (*f)(void *)) {
//   if (generator_stack.count == 0) {
//     Generator *g = malloc(sizeof(Generator));
//     assert(g != NULL && "Buy more RAM lol");
//     memset(g, 0, sizeof(*g));
//     da_append(&generator_stack, g);
//   }
//   Generator *g = malloc(sizeof(Generator));
//   assert(g != NULL && "Buy more RAM lol");
//   memset(g, 0, sizeof(*g));
//
//   g->stack_base =
//       mmap(NULL, GENERATOR_STACK_CAPACITY, PROT_WRITE | PROT_READ,
//            MAP_PRIVATE | MAP_STACK | MAP_ANONYMOUS | MAP_GROWSDOWN, -1, 0);
//   assert(g->stack_base != MAP_FAILED);
//   void **rsp = (void **)((char *)g->stack_base + GENERATOR_STACK_CAPACITY);
//   *(--rsp) = generator__finish_current;
//   *(--rsp) = f;
//   *(--rsp) = 0; // push rdi
//   *(--rsp) = 0; // push rbx
//   *(--rsp) = 0; // push rbp
//   *(--rsp) = 0; // push r12
//   *(--rsp) = 0; // push r13
//   *(--rsp) = 0; // push r14
//   *(--rsp) = 0; // push r15
//   g->rsp = rsp;
//   g->fresh = true;
//   return g;
// }
//
// void generator_destroy(Generator *g) {
//   munmap(g->stack_base, GENERATOR_STACK_CAPACITY);
//   free(g);
// }
//
