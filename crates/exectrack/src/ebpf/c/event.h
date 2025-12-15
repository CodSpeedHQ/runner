#ifndef __EXECTRACK_EVENT_H__
#define __EXECTRACK_EVENT_H__

#define EVENT_TYPE_FORK 1
#define EVENT_TYPE_EXEC 2
#define EVENT_TYPE_EXIT 3

/* Event structure - shared between BPF and userspace */
struct event {
    uint8_t event_type; /* See EVENT_TYPE_* constants above */
    uint64_t timestamp; /* monotonic time in nanoseconds (CLOCK_MONOTONIC) */
    uint32_t pid;       /* Process ID */
    uint32_t tid;       /* Thread ID */
    uint32_t ppid;      /* Parent Process ID (for fork events) */
    char comm[16];      /* Command name (null-terminated) */
};

#endif /* __EXECTRACK_EVENT_H__ */
