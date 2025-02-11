#ifdef __x86_64__
void __attribute__((naked)) _yield_coroutine(void)
{
    asm(
    "    pushq %rdi\n"
    "    pushq %rbp\n"
    "    pushq %rbx\n"
    "    pushq %r12\n"
    "    pushq %r13\n"
    "    pushq %r14\n"
    "    pushq %r15\n"
    "    movq %rsp, %rdi\n"     // rsp
    "    movq $0, %rsi\n"       // sm = SM_NONE
    "    jmp switch_context\n");
}

void __attribute__((naked)) _sleep_read(int fd)
{
    asm(
    "    pushq %rdi\n"
    "    pushq %rbp\n"
    "    pushq %rbx\n"
    "    pushq %r12\n"
    "    pushq %r13\n"
    "    pushq %r14\n"
    "    pushq %r15\n"
    "    movq %rdi, %rdx\n"     // fd
    "    movq %rsp, %rdi\n"     // rsp
    "    movq $1, %rsi\n"       // sm = SM_READ
    "    jmp switch_context\n");
}

void __attribute__((naked)) _sleep_write(int fd)
{
    asm(
    "    pushq %rdi\n"
    "    pushq %rbp\n"
    "    pushq %rbx\n"
    "    pushq %r12\n"
    "    pushq %r13\n"
    "    pushq %r14\n"
    "    pushq %r15\n"
    "    movq %rdi, %rdx\n"     // fd
    "    movq %rsp, %rdi\n"     // rsp
    "    movq $2, %rsi\n"       // sm = SM_WRITE
    "    jmp switch_context\n");
}

void __attribute__((naked)) _restore_coroutine(void *rsp)
{
    asm(
    "    movq %rdi, %rsp\n"
    "    popq %r15\n"
    "    popq %r14\n"
    "    popq %r13\n"
    "    popq %r12\n"
    "    popq %rbx\n"
    "    popq %rbp\n"
    "    popq %rdi\n"
    "    ret\n");
}
#endif // __x86_64__

#ifdef __aarch64__

void __attribute__((naked)) _yield_coroutine(void)
{
    asm(
    "    stp x29, x30, [sp, #-16]!\n"
    "    mov x0, sp\n"
    "    mov x1, #0\n"
    "    bl switch_context\n"
    "    ldp x29, x30, [sp], #16\n"
    "    ret\n");
}

void __attribute__((naked)) _sleep_read(int fd)
{
    asm(
    "    stp x29, x30, [sp, #-16]!\n"
    "    mov x2, x0\n"
    "    mov x0, sp\n"
    "    mov x1, #1\n"
    "    bl switch_context\n"
    "    ldp x29, x30, [sp], #16\n"
    "    ret\n");
}

void __attribute__((naked)) _sleep_write(int fd)
{
    asm(
    "    stp x29, x30, [sp, #-16]!\n"
    "    mov x2, x0\n"
    "    mov x0, sp\n"
    "    mov x1, #2\n"
    "    bl switch_context\n"
    "    ldp x29, x30, [sp], #16\n"
    "    ret\n");
}

void __attribute__((naked)) _restore_context(void *rsp)
{
    (void)rsp;
    asm(
    "    mov sp, x0\n"
    "    ldp x29, x30, [sp], #16\n"
    "    ret\n");
}

void __attribute__((naked)) _switch_context(void *rsp, Sleep_Mode sm, int fd)
{
    asm(
    "    mov sp, x0\n"
    "    mov x2, x1\n"
    "    mov x3, x2\n"
    "    bl switch_context\n"
    "    ret\n");
}

#endif // __aarch64__
