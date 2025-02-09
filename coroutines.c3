// coroutine.c3
// Coroutine library for C3 which does not work since the lanaugage
// does not fully support inline assembly, specifically the `jmp` instruction.
import std::io;
import libc;

// Constants
const STACK_CAPACITY = 20 * 1024 * 1024;
const DA_INIT_CAP = 256;
const POLLRDNORM = 0x040;
const POLLWRNORM = 0x100;

// Poll structure
struct PollFD
{
    int fd;
    short events;
    short revents;
}

// Extern functions
extern fn int poll(PollFD* fds, ulong nfds, int timeout);

// Context structures
struct Context
{
    void* rsp;
    void* stack_base;
    bool dead;
}

struct Contexts
{
    Context* items;
    usz count;
    usz capacity;
}

struct Indices
{
    usz* items;
    usz count;
    usz capacity;
}

// Sleep structure
struct Sleep
{
    int key;
    usz value;
}

struct Sleeps
{
    Sleep* items;
    usz count;
    usz capacity;
}

struct Polls
{
    PollFD* items;
    usz count;
    usz capacity;
}

// Macros
macro void @da_append(#da, #item)
{
    if (#da.count >= #da.capacity)
    {
        #da.capacity = #da.capacity == 0 ? DA_INIT_CAP : #da.capacity * 2;
        #da.items = realloc(#da.items, #da.capacity * $sizeof(#item));
        assert(#da.items != null, "Buy more RAM lol");
    }
    #da.items[#da.count++] = #item;
}

macro void @da_remove_unordered(#da, #i)
{
    usz j = #i;
	assert(j < #da.count);
	#da.items[j] = #da.items[--#da.count];
}

macro void @todo(#message)
{
    io::eprintf("%s:%d: TODO: %s\n", $$FILE, $$LINE, #message);
    libc::abort();
}

macro void @unreachable(#message)
{
    io::stderr().printf("%s:%d: UNREACHABLE: %s\n", $file, $line, message);
    std::libc::abort();
}

// Global state
usz current = 0;
Indices active = Indices { null, 0, 0 };
Indices dead = Indices { null, 0, 0 };
Contexts contexts_ = Contexts { null, 0, 0 };
Indices asleep = Indices { null, 0, 0 };
Polls polls = Polls { null, 0, 0 };

// Assembly functions
fn void coroutine_yield() @naked
{
    asm
    {
        pushq $rdi;
        pushq $rbp;
        pushq $rbx;
        pushq $r12;
        pushq $r13;
        pushq $r14;
        pushq $r15;
        movq $rdi, $rsp;  // Pass current stack pointer via RDI
        movq $rsi, 0;     // SM_NONE via RSI
        jmp coroutine_switch_context;
    }
}

fn void coroutine_sleep_read(int fd) @naked
{
    asm
    {
        pushq $rdi;
        pushq $rbp;
        pushq $rbx;
        pushq $r12;
        pushq $r13;
        pushq $r14;
        // pushq $r15;
        movq $rdx, $rdi;     // fd parameter
        movq $rdi, $rsp;     // Save stack pointer
        movq $rsi, 1;        // sm = SM_READ
        jmp coroutine_switch_context;
    }
}

fn void coroutine_sleep_write(int fd) @naked
{
    asm
    {
        pushq $rdi;
        pushq $rbp;
        pushq $rbx;
        pushq $r12;
        pushq $r13;
        pushq $r14;
        // pushq $r15;
        movq $rdx, $rdi;     // fd parameter
        movq $rdi, $rsp;     // Save stack pointer
        movq $rsi, 2;        // sm = SM_WRITE
        jmp coroutine_switch_context;
    }
}

fn void coroutine_restore_context(void* rsp) @naked
{
    asm
    {
        movq $rsp, $rdi;     // Restore stack pointer from parameter
        // popq $r15;
        popq $r14;
        popq $r13;
        popq $r12;
        popq $rbx;
        popq $rbp;
        popq $rdi;
        ret;
    }
}

enum Sleep_Mode
{
    SM_NONE,
    SM_READ,
    SM_WRITE,
}

fn void coroutine_switch_context(void* rsp, Sleep_Mode sm, int fd)
{
    contexts_.items[active.items[current]].rsp = rsp;

    switch (sm)
    {
        case SM_NONE: current += 1; break;
        case SM_READ:
            @da_append(&asleep, active.items[current]);
            PollFD pfd = { .fd = fd, .events = POLLRDNORM };
            @da_append(&polls, pfd);
            @da_remove_unordered( &active, current);
            break;

        case SM_WRITE:
            @da_append(&asleep, active.items[current]);
            PollFD pfd = { .fd = fd, .events = POLLWRNORM };
            @da_append(&polls, pfd);
            @da_remove_unordered( &active, current);
            break;

        default: unreachable("coroutine_switch_context");
    }

    if (polls.count > 0)
    {
        int timeout = active.count == 0 ? -1 : 0;
        int result = poll(polls.items, polls.count, timeout);
        if (result < 0) @todo("poll");

        usz i = 0;
        while (i < polls.count)
        {
            if (polls.items[i].revents != 0)
            {
                usz id = asleep.items[i];
                @da_remove_unordered( &polls, i);
                @da_remove_unordered( &asleep, i);
                @da_append(&active, id);
            }
            else
            {
                i += 1;
            }
        }
    }

    assert(active.count > 0);
    current %= active.count;
    coroutine_restore_context(contexts_.items[active.items[current]].rsp);
}

fn void coroutine_init()
{
    @da_append(&contexts_, Context { });
    @da_append(&active, 0);
}

fn void coroutine_finish()
{
    if (active.items[current] == 0)
    {
        for (usz i = 1; i < contexts_.count; ++i)
        {
            free(contexts_.items[i].stack_base);
        }
        free(contexts_.items);
        free(active.items);
        free(dead.items);
        free(polls.items);
        free(asleep.items);
        contexts_ = Contexts { null, 0, 0 };
        active = Indices { null, 0, 0 };
        dead = Indices { null, 0, 0 };
        polls = Polls { null, 0, 0 };
        asleep = Indices { null, 0, 0 };
        return;
    }

    contexts_.items[active.items[current]].dead = true;
    @da_append(&dead, active.items[current]);
    @da_remove_unordered( &active, current);

    if (polls.count > 0)
    {
        int timeout = active.count == 0 ? -1 : 0;
        int result = poll(polls.items, polls.count, timeout);
        if (result < 0) @todo("poll");

        usz i = 0;
        while (i < polls.count)
        {
            if (polls.items[i].revents != 0)
            {
                usz id = asleep.items[i];
                @da_remove_unordered( &polls, i);
                @da_remove_unordered( &asleep, i);
                @da_append(&active, id);
            }
            else
            {
                i += 1;
            }
        }
    }

    assert(active.count > 0);
    current %= active.count;
    coroutine_restore_context(contexts_.items[active.items[current]].rsp);
}

def CoroutineFn = fn void(void*);
fn void go(CoroutineFn f, void* arg = null)
{
    usz id;
    if (dead.count > 0)
    {
        id = dead.items[--dead.count];
        assert(contexts_.items[id].dead);
        contexts_.items[id].dead = false;
    }
    else
    {
        @da_append(&contexts_, Context { });
        id = contexts_.count - 1;
        contexts_.items[id].stack_base = malloc(STACK_CAPACITY);
    }

    void** rsp = (void**)((char*)contexts_.items[id].stack_base + STACK_CAPACITY);
    *(--rsp) = (void*)&coroutine_finish;
    *(--rsp) = (void*)f;
    *(--rsp) = arg;     // rdi
    *(--rsp) = null;    // rbx
    *(--rsp) = null;    // rbp
    *(--rsp) = null;    // r12
    *(--rsp) = null;    // r13
    *(--rsp) = null;    // r14
    *(--rsp) = null;    // r15
    contexts_.items[id].rsp = rsp;

    @da_append(&active, id);
}

fn usz coroutine_id()
{
    return active.items[current];
}

fn usz coroutine_alive()
{
    return active.count;
}
