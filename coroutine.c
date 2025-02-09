// Copyright 2025 Alexey Kutepov <reximkut@gmail.com>
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
// LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
// ------------------------------------------------------------------------

#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <poll.h>
#include <sys/mman.h>
#include <unistd.h>

// TODO: make the STACK_CAPACITY customizable by the user
// #define STACK_CAPACITY (4*1024)
#define STACK_CAPACITY (1024 * getpagesize())

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

#define UNUSED(x) (void)(x)
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

typedef struct {
  void *rsp;
  void *stack_base;
} Context;

typedef struct {
  Context *items;
  size_t count;
  size_t capacity;
} Contexts;

typedef struct {
  size_t *items;
  size_t count;
  size_t capacity;
} Indices;

typedef struct {
  struct pollfd *items;
  size_t count;
  size_t capacity;
} Polls;

// TODO: coroutines library probably does not work well in multithreaded
// environment
static size_t current = 0;
static Indices active = {0};
static Indices dead = {0};
static Contexts contexts = {0};
static Indices asleep = {0};
static Polls polls = {0};

// TODO: ARM support
//   Requires modifications in all the @arch places

typedef enum {
  SM_NONE = 0,
  SM_READ,
  SM_WRITE,
} Sleep_Mode;

extern void coroutine_restore_context(void *rsp);

void coroutine_switch_context(void *rsp, Sleep_Mode sm, int fd) {
  contexts.items[active.items[current]].rsp = rsp;

  switch (sm) {
  case SM_NONE:
    current += 1;
    break;
  case SM_READ: {
    da_append(&asleep, active.items[current]);
    struct pollfd pfd = {
        .fd = fd,
        .events = POLLRDNORM,
    };
    da_append(&polls, pfd);
    da_remove_unordered(&active, current);
  } break;

  case SM_WRITE: {
    da_append(&asleep, active.items[current]);
    struct pollfd pfd = {
        .fd = fd,
        .events = POLLWRNORM,
    };
    da_append(&polls, pfd);
    da_remove_unordered(&active, current);
  } break;

  default:
    UNREACHABLE("coroutine_switch_context");
  }

  if (polls.count > 0) {
    int timeout = active.count == 0 ? -1 : 0;
    int result = poll(polls.items, polls.count, timeout);
    if (result < 0)
      TODO("poll");

    for (size_t i = 0; i < polls.count;) {
      if (polls.items[i].revents) {
        size_t id = asleep.items[i];
        da_remove_unordered(&polls, i);
        da_remove_unordered(&asleep, i);
        da_append(&active, id);
      } else {
        ++i;
      }
    }
  }

  assert(active.count > 0);
  current %= active.count;
  coroutine_restore_context(contexts.items[active.items[current]].rsp);
}

// TODO: think how to get rid of coroutine_init() call at all
void coroutine_init(void) {
  if (contexts.count != 0)
    return;
  da_append(&contexts, (Context){0});
  da_append(&active, 0);
}

void finish_current(void) {
  if (active.items[current] == 0) {
    UNREACHABLE("Main Coroutine with id == 0 should never reach this place");
  }

  da_append(&dead, active.items[current]);
  da_remove_unordered(&active, current);

  if (polls.count > 0) {
    int timeout = active.count == 0 ? -1 : 0;
    int result = poll(polls.items, polls.count, timeout);
    if (result < 0)
      TODO("poll");

    for (size_t i = 0; i < polls.count;) {
      if (polls.items[i].revents) {
        size_t id = asleep.items[i];
        da_remove_unordered(&polls, i);
        da_remove_unordered(&asleep, i);
        da_append(&active, id);
      } else {
        ++i;
      }
    }
  }

  assert(active.count > 0);
  current %= active.count;
  coroutine_restore_context(contexts.items[active.items[current]].rsp);
}

void _go(void (*f)(void *), void *arg) {
  if (contexts.count == 0) {
    da_append(&contexts, (Context){0});
    da_append(&active, 0);
  }

  size_t id;
  if (dead.count > 0) {
    id = dead.items[--dead.count];
  } else {
    da_append(&contexts, ((Context){0}));
    id = contexts.count - 1;
    contexts.items[id].stack_base =
        mmap(NULL, STACK_CAPACITY, PROT_WRITE | PROT_READ,
             MAP_PRIVATE | MAP_STACK | MAP_ANONYMOUS | MAP_GROWSDOWN, -1, 0);
    assert(contexts.items[id].stack_base != MAP_FAILED);
  }

  void **rsp =
      (void **)((char *)contexts.items[id].stack_base + STACK_CAPACITY);
  // @arch
  *(--rsp) = finish_current;
  *(--rsp) = f;
  *(--rsp) = arg; // push rdi
  *(--rsp) = 0;   // push rbx
  *(--rsp) = 0;   // push rbp
  *(--rsp) = 0;   // push r12
  *(--rsp) = 0;   // push r13
  *(--rsp) = 0;   // push r14
  *(--rsp) = 0;   // push r15
  contexts.items[id].rsp = rsp;

  da_append(&active, id);
}

size_t _id(void) { return active.items[current]; }

size_t _alive(void) { return active.count; }

void _wake_up(size_t id) {
  // @speed coroutine_wake_up is linear
  for (size_t i = 0; i < asleep.count; ++i) {
    if (asleep.items[i] == id) {
      da_remove_unordered(&asleep, id);
      da_remove_unordered(&polls, id);
      da_append(&active, id);
      return;
    }
  }
}
