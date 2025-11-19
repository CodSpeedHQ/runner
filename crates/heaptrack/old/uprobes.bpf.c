/* clang-format off */

/* These header file should be included first and in sequence,
 * because our following included file may depend on these. Turn
 * off clang-format to achieve this purpose. */
#include "vmlinux.h"
#include <bpf/bpf_core_read.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
/* clang-format on */

#include "event.h"
#include "common.h"

char LICENSE[] SEC("license") = "GPL";

/* Map to store malloc size between entry and return */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, __u64);   /* tid */
    __type(value, __u64); /* size */
} malloc_size SEC(".maps");

SEC("uprobe")
int uprobe_malloc(struct pt_regs *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    /* Check if this PID is being tracked */
    if (is_tracked(pid)) {
        /* Store the size argument for the return probe */
        __u64 size = PT_REGS_PARM1(ctx);
        bpf_map_update_elem(&malloc_size, &tid, &size, BPF_ANY);
    }

    return 0;
}

SEC("uretprobe")
int uretprobe_malloc(struct pt_regs *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    /* Check if this PID is being tracked */
    if (is_tracked(pid)) {
        __u64 *size_ptr = bpf_map_lookup_elem(&malloc_size, &tid);
        if (size_ptr) {
            __u64 addr = PT_REGS_RC(ctx);
            __u64 size = *size_ptr;

            /* Only report successful allocations */
            if (addr != 0) {
                struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
                if (e) {
                    e->timestamp = bpf_ktime_get_ns();
                    e->pid = pid;
                    e->event_type = EVENT_TYPE_MALLOC;
                    e->addr = addr;
                    e->size = size;
                    bpf_ringbuf_submit(e, 0);
                }
            }

            bpf_map_delete_elem(&malloc_size, &tid);
        }
    }

    return 0;
}

SEC("uprobe")
int uprobe_free(struct pt_regs *ctx) {
    __u32 pid = bpf_get_current_pid_tgid() >> 32;

    /* Check if this PID is being tracked */
    if (is_tracked(pid)) {
        __u64 addr = PT_REGS_PARM1(ctx);

        /* Only track non-NULL frees */
        if (addr != 0) {
            struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
            if (e) {
                e->timestamp = bpf_ktime_get_ns();
                e->pid = pid;
                e->event_type = EVENT_TYPE_FREE;
                e->addr = addr;
                e->size = 0;  /* size unknown for free */
                bpf_ringbuf_submit(e, 0);
            }
        }
    }

    return 0;
}

/* Map to store calloc parameters between entry and return */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, __u64);   /* tid */
    __type(value, __u64); /* nmemb * size (total size) */
} calloc_size SEC(".maps");

SEC("uprobe")
int uprobe_calloc(struct pt_regs *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    /* Check if this PID is being tracked */
    if (is_tracked(pid)) {
        /* calloc(nmemb, size) - calculate total size */
        __u64 nmemb = PT_REGS_PARM1(ctx);
        __u64 size = PT_REGS_PARM2(ctx);
        __u64 total_size = nmemb * size;
        bpf_map_update_elem(&calloc_size, &tid, &total_size, BPF_ANY);
    }

    return 0;
}

SEC("uretprobe")
int uretprobe_calloc(struct pt_regs *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    /* Check if this PID is being tracked */
    if (is_tracked(pid)) {
        __u64 *size_ptr = bpf_map_lookup_elem(&calloc_size, &tid);
        if (size_ptr) {
            __u64 addr = PT_REGS_RC(ctx);
            __u64 size = *size_ptr;

            /* Only report successful allocations */
            if (addr != 0) {
                struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
                if (e) {
                    e->timestamp = bpf_ktime_get_ns();
                    e->pid = pid;
                    e->event_type = EVENT_TYPE_CALLOC;
                    e->addr = addr;
                    e->size = size;
                    bpf_ringbuf_submit(e, 0);
                }
            }

            bpf_map_delete_elem(&calloc_size, &tid);
        }
    }

    return 0;
}

/* Map to store realloc parameters between entry and return */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, __u64);   /* tid */
    __type(value, __u64); /* size */
} realloc_size SEC(".maps");

SEC("uprobe")
int uprobe_realloc(struct pt_regs *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    /* Check if this PID is being tracked */
    if (is_tracked(pid)) {
        /* realloc(ptr, size) - we only care about size */
        __u64 size = PT_REGS_PARM2(ctx);
        bpf_map_update_elem(&realloc_size, &tid, &size, BPF_ANY);
    }

    return 0;
}

SEC("uretprobe")
int uretprobe_realloc(struct pt_regs *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    /* Check if this PID is being tracked */
    if (is_tracked(pid)) {
        __u64 *size_ptr = bpf_map_lookup_elem(&realloc_size, &tid);
        if (size_ptr) {
            __u64 addr = PT_REGS_RC(ctx);
            __u64 size = *size_ptr;

            /* Only report successful reallocations */
            if (addr != 0) {
                struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
                if (e) {
                    e->timestamp = bpf_ktime_get_ns();
                    e->pid = pid;
                    e->event_type = EVENT_TYPE_REALLOC;
                    e->addr = addr;
                    e->size = size;
                    bpf_ringbuf_submit(e, 0);
                }
            }

            bpf_map_delete_elem(&realloc_size, &tid);
        }
    }

    return 0;
}

/* Map to store aligned_alloc parameters between entry and return */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, __u64);   /* tid */
    __type(value, __u64); /* size */
} aligned_alloc_size SEC(".maps");

SEC("uprobe")
int uprobe_aligned_alloc(struct pt_regs *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    /* Check if this PID is being tracked */
    if (is_tracked(pid)) {
        /* aligned_alloc(alignment, size) - we only care about size */
        __u64 size = PT_REGS_PARM2(ctx);
        bpf_map_update_elem(&aligned_alloc_size, &tid, &size, BPF_ANY);
    }

    return 0;
}

SEC("uretprobe")
int uretprobe_aligned_alloc(struct pt_regs *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    /* Check if this PID is being tracked */
    if (is_tracked(pid)) {
        __u64 *size_ptr = bpf_map_lookup_elem(&aligned_alloc_size, &tid);
        if (size_ptr) {
            __u64 addr = PT_REGS_RC(ctx);
            __u64 size = *size_ptr;

            /* Only report successful allocations */
            if (addr != 0) {
                struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
                if (e) {
                    e->timestamp = bpf_ktime_get_ns();
                    e->pid = pid;
                    e->event_type = EVENT_TYPE_ALIGNED_ALLOC;
                    e->addr = addr;
                    e->size = size;
                    bpf_ringbuf_submit(e, 0);
                }
            }

            bpf_map_delete_elem(&aligned_alloc_size, &tid);
        }
    }

    return 0;
}
