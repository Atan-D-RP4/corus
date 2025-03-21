#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <threads.h>

#include <sys/mman.h>
#include <unistd.h>

#define da_last(da) (da)->items[(da)->count - 1]

// Initial capacity of a dynamic array
#ifndef DA_INIT_CAP
#define DA_INIT_CAP 256
#endif

// Append an item to a dynamic array
#define da_append(da, item)                                                    \
  do {                                                                         \
    if ((da)->count >= (da)->capacity) {                                       \
      (da)->capacity = (da)->capacity == 0 ? DA_INIT_CAP : (da)->capacity * 2; \
      (da)->items =                                                            \
          realloc((da)->items, (da)->capacity * sizeof(*(da)->items));         \
      assert((da)->items != NULL && "Buy more RAM lol");                       \
    }                                                                          \
                                                                               \
    (da)->items[(da)->count++] = (item);                                       \
  } while (0)

#define da_remove_unordered(da, i)                                             \
  do {                                                                         \
    size_t j = (i);                                                            \
    assert(j < (da)->count);                                                   \
    (da)->items[j] = (da)->items[--(da)->count];                               \
  } while (0)

#define TODO(message)                                                          \
  do {                                                                         \
    fprintf(stderr, "%s:%d: TODO: %s\n", __FILE__, __LINE__, message);         \
    abort();                                                                   \
  } while (0)
#define UNREACHABLE(message)                                                   \
  do {                                                                         \
    fprintf(stderr, "%s:%d: UNREACHABLE: %s\n", __FILE__, __LINE__, message);  \
    abort();                                                                   \
  } while (0)

#define GENERATOR_STACK_CAPACITY (1024 * getpagesize())

typedef struct {
  void *rsp;
  void *stack_base;
  bool dead;
  bool fresh;
} Generator;

typedef struct {
  Generator **items;
  size_t count;
  size_t capacity;
} Generator_Stack;

#define foreach(it, g, arg)                                                    \
  for (void *it = _next((void *)g, arg); !(g)->dead; it = _next(g, arg))

thread_local Generator_Stack generator_stack = {0};

// Linux x86_64 call convention
// %rdi, %rsi, %rdx, %rcx, %r8, and %r9

extern void * _next(void *g, void *arg);
extern void _restore_context(void *rsp);
extern void _restore_context_with_return(void *rsp, void *arg);
extern void * _yield(void *arg);

void _switch_context(Generator *g, void *arg, void *rsp) {
  da_last(&generator_stack)->rsp = rsp;
  da_append(&generator_stack, g);
  if (g->fresh) {
    g->fresh = false;
    void **rsp = (void **)((char *)g->stack_base + GENERATOR_STACK_CAPACITY);
    *(rsp - 3) = arg;
    _restore_context(g->rsp);
  } else {
    _restore_context_with_return(g->rsp, arg);
  }
}

void _return(void *arg, void *rsp) {
  da_last(&generator_stack)->rsp = rsp;
  generator_stack.count -= 1;
  _restore_context_with_return(da_last(&generator_stack)->rsp, arg);
}

void generator__finish_current(void) {
  da_last(&generator_stack)->dead = true;
  generator_stack.count -= 1;
  _restore_context_with_return(da_last(&generator_stack)->rsp, NULL);
}

Generator *generator_create(void (*f)(void *)) {
  if (generator_stack.count == 0) {
    Generator *g = malloc(sizeof(Generator));
    assert(g != NULL && "Buy more RAM lol");
    memset(g, 0, sizeof(*g));
    da_append(&generator_stack, g);
  }
  Generator *g = malloc(sizeof(Generator));
  assert(g != NULL && "Buy more RAM lol");
  memset(g, 0, sizeof(*g));

  g->stack_base =
      mmap(NULL, GENERATOR_STACK_CAPACITY, PROT_WRITE | PROT_READ,
           MAP_PRIVATE | MAP_STACK | MAP_ANONYMOUS | MAP_GROWSDOWN, -1, 0);
  assert(g->stack_base != MAP_FAILED);
  void **rsp = (void **)((char *)g->stack_base + GENERATOR_STACK_CAPACITY);
  *(--rsp) = generator__finish_current;
  *(--rsp) = f;
  *(--rsp) = 0; // push rdi
  *(--rsp) = 0; // push rbx
  *(--rsp) = 0; // push rbp
  *(--rsp) = 0; // push r12
  *(--rsp) = 0; // push r13
  *(--rsp) = 0; // push r14
  *(--rsp) = 0; // push r15
  g->rsp = rsp;
  g->fresh = true;
  return g;
}

void generator_destroy(Generator *g) {
  munmap(g->stack_base, GENERATOR_STACK_CAPACITY);
  free(g);
}

void fib(void *arg) {
  long max = (long)arg;
  long a = 0;
  long b = 1;
  while (a < max) {
    _yield((void *)a);
    long c = a + b;
    a = b;
    b = c;
  }
}

int main() {
  Generator *g = generator_create(fib);
  foreach (value, g, (void *)(1000 * 1000)) {
    printf("%ld\n", (long)value);
  }
  generator_destroy(g);
}
