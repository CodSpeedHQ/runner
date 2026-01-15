#ifndef __EVENT_H__
#define __EVENT_H__

#define EVENT_TYPE_MALLOC 1
#define EVENT_TYPE_FREE 2
#define EVENT_TYPE_CALLOC 3
#define EVENT_TYPE_REALLOC 4
#define EVENT_TYPE_ALIGNED_ALLOC 5
#define EVENT_TYPE_MMAP 6
#define EVENT_TYPE_MUNMAP 7
#define EVENT_TYPE_BRK 8

/* Event structure - shared between BPF and userspace */
struct event {
    uint8_t event_type; /* See EVENT_TYPE_* constants above */
    uint64_t timestamp; /* monotonic time in nanoseconds (CLOCK_MONOTONIC) */
    uint32_t pid;
    uint32_t tid;
    uint64_t addr; /* address returned/freed */
    uint64_t size; /* size requested */
};

#endif /* __EVENT_H__ */
