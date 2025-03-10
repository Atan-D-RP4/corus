#ifdef __x86_64__

void *__attribute__((naked)) _next(void *g, void *arg) {
  // @arch
  asm("    testq %rdi, %rdi\n"
      "    jnz 2f\n"
      "    xor %rax, %rax\n"
      "    ret\n"
      "2:\n"
      "    pushq %rdi\n"
      "    pushq %rbp\n"
      "    pushq %rbx\n"
      "    pushq %r12\n"
      "    pushq %r13\n"
      "    pushq %r14\n"
      "    pushq %r15\n"
      "    movq %rsp, %rdx\n" // rsp
      "    jmp _switch_context\n");
}

void __attribute__((naked)) _restore_context(void *rsp) {
  // @arch
  asm("    movq %rdi, %rsp\n"
      "    popq %r15\n"
      "    popq %r14\n"
      "    popq %r13\n"
      "    popq %r12\n"
      "    popq %rbx\n"
      "    popq %rbp\n"
      "    popq %rdi\n"
      "    ret\n");
}

void __attribute__((naked)) _restore_context_with_return(void *rsp,
                                                                  void *arg) {
  // @arch
  asm("    movq %rdi, %rsp\n"
      "    movq %rsi, %rax\n"
      "    popq %r15\n"
      "    popq %r14\n"
      "    popq %r13\n"
      "    popq %r12\n"
      "    popq %rbx\n"
      "    popq %rbp\n"
      "    popq %rdi\n"
      "    ret\n");
}

void *__attribute__((naked)) _yield(void *arg) {
  // @arch
  asm("    pushq %rdi\n"
      "    pushq %rbp\n"
      "    pushq %rbx\n"
      "    pushq %r12\n"
      "    pushq %r13\n"
      "    pushq %r14\n"
      "    pushq %r15\n"
      "    movq %rsp, %rsi\n" // rsp
      "    jmp _return\n");
}


#endif // __x86_64__

#ifdef __aarch64__

void *__attribute__((naked)) _generator_next(void *g, void *arg) {
  asm volatile("cbz x0, 1f\n"
               "stp x0, x29, [sp, #-16]!\n"
               "stp x19, x20, [sp, #-16]!\n"
               "stp x21, x22, [sp, #-16]!\n"
               "stp x23, x30, [sp, #-16]!\n"
               "mov x2, sp\n"
               "b _switch_context\n"
               "1:\n"
               "mov x0, #0\n"
               "ret\n");
}

void __attribute__((naked)) _generator_restore_context(void *rsp) {
  asm volatile("mov sp, x0\n"
               "ldp x23, x30, [sp], #16\n"
               "ldp x21, x22, [sp], #16\n"
               "ldp x19, x20, [sp], #16\n"
               "ldp x0, x29, [sp], #16\n"
               "ret\n");
}

void __attribute__((naked)) _generator_restore_context_with_return(void *rsp,
                                                                   void *arg) {
  asm volatile("mov sp, x0\n"
               "mov x2, x1\n"
               "ldp x23, x30, [sp], #16\n"
               "ldp x21, x22, [sp], #16\n"
               "ldp x19, x20, [sp], #16\n"
               "ldp x0, x29, [sp], #16\n"
               "mov x0, x2\n"
               "ret\n");
}

void *__attribute__((naked)) _generator_yield(void *arg) {
  asm volatile("stp x0, x29, [sp, #-16]!\n"
               "stp x19, x20, [sp, #-16]!\n"
               "stp x21, x22, [sp, #-16]!\n"
               "stp x23, x30, [sp, #-16]!\n"
               "mov x1, sp\n"
               "b _return\n");
}

#endif // __aarch64__
