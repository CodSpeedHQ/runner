#include "vmlinux.h"
#include <bpf/bpf_core_read.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

#include "event.h"

char LICENSE[] SEC("license") = "GPL";

/* Map to store PIDs we're tracking */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, __u32);   /* PID */
    __type(value, __u8);  /* marker (always 1 if present) */
} tracked_pids SEC(".maps");

/* Map to store parent-child relationships to detect hierarchy */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, __u32);   /* child PID */
    __type(value, __u32); /* parent PID */
} pids_ppid SEC(".maps");

/* Ring buffer for sending events to userspace */
struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 256 * 1024);
} events SEC(".maps");

/* Map to control whether tracking is enabled (0 = disabled, 1 = enabled) */
struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, __u8);
} tracking_enabled SEC(".maps");

/* Helper to check if tracking is currently enabled */
static __always_inline int is_enabled(void) {
    __u32 key = 0;
    __u8 *enabled = bpf_map_lookup_elem(&tracking_enabled, &key);

    /* Default to enabled if map not initialized */
    if (!enabled) {
        return 1;
    }

    return *enabled;
}

/* Helper to check if a PID or any of its ancestors should be tracked */
static __always_inline int is_tracked(__u32 pid) {
    /* Direct check */
    if (bpf_map_lookup_elem(&tracked_pids, &pid)) {
        return 1;
    }

    /* Check parent recursively (up to 5 levels) */
    #pragma unroll
    for (int i = 0; i < 5; i++) {
        __u32 *ppid = bpf_map_lookup_elem(&pids_ppid, &pid);
        if (!ppid) {
            break;
        }
        pid = *ppid;
        if (bpf_map_lookup_elem(&tracked_pids, &pid)) {
            return 1;
        }
    }

    return 0;
}

SEC("tracepoint/sched/sched_process_fork")
int tracepoint_sched_fork(struct trace_event_raw_sched_process_fork *ctx) {
    __u32 parent_pid = ctx->parent_pid;
    __u32 child_pid = ctx->child_pid;

    /* Print process fork with PIDs */
    // bpf_printk("sched_fork: parent_pid=%u child_pid=%u", parent_pid, child_pid);

    /* Check if parent is being tracked */
    if (is_tracked(parent_pid)) {
        /* Auto-track this child */
        __u8 marker = 1;
        bpf_map_update_elem(&tracked_pids, &child_pid, &marker, BPF_ANY);
        bpf_map_update_elem(&pids_ppid, &child_pid, &parent_pid, BPF_ANY);

        // bpf_printk("auto-tracking child process: child_pid=%u", child_pid);
    }

    return 0;
}

SEC("tracepoint/syscalls/sys_enter_execve")
int tracepoint_sys_execve(struct trace_event_raw_sys_enter *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    /* Check if this process or any parent is being tracked */
    if (is_tracked(pid) && is_enabled()) {
        struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
        if (e) {
            e->timestamp = bpf_ktime_get_ns();
            e->pid = pid;
            e->tid = tid & 0xFFFFFFFF;
            e->event_type = EVENT_TYPE_EXECVE;
            e->addr = 0;
            e->size = 0;
            bpf_ringbuf_submit(e, 0);
        }
    }

    return 0;
}

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
    if (is_tracked(pid) && is_enabled()) {
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
                    e->tid = tid & 0xFFFFFFFF;
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
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    /* Check if this PID is being tracked */
    if (is_tracked(pid) && is_enabled()) {
        __u64 addr = PT_REGS_PARM1(ctx);

        /* Only track non-NULL frees */
        if (addr != 0) {
            struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
            if (e) {
                e->timestamp = bpf_ktime_get_ns();
                e->pid = pid;
                e->tid = tid & 0xFFFFFFFF;
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
    if (is_tracked(pid) && is_enabled()) {
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
                    e->tid = tid & 0xFFFFFFFF;
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
    if (is_tracked(pid) && is_enabled()) {
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
                    e->tid = tid & 0xFFFFFFFF;
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
    if (is_tracked(pid) && is_enabled()) {
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
                    e->tid = tid & 0xFFFFFFFF;
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

/* Map to store mmap parameters between entry and return */
struct mmap_args {
    __u64 addr;
    __u64 len;
};

struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, __u64);   /* tid */
    __type(value, struct mmap_args);
} mmap_temp SEC(".maps");

SEC("tracepoint/syscalls/sys_enter_mmap")
int tracepoint_sys_enter_mmap(struct trace_event_raw_sys_enter *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    if (is_tracked(pid)) {
        struct mmap_args args = {0};

        /* mmap(addr, len, prot, flags, fd, offset)
         * We care about addr (can be 0 for kernel choice) and len */
        args.addr = ctx->args[0];
        args.len = ctx->args[1];

        bpf_map_update_elem(&mmap_temp, &tid, &args, BPF_ANY);
    }

    return 0;
}

SEC("tracepoint/syscalls/sys_exit_mmap")
int tracepoint_sys_exit_mmap(struct trace_event_raw_sys_exit *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    if (is_tracked(pid) && is_enabled()) {
        struct mmap_args *args = bpf_map_lookup_elem(&mmap_temp, &tid);
        if (args) {
            __s64 ret = ctx->ret;

            /* Only track successful mmap calls (ret != MAP_FAILED which is -1) */
            if (ret > 0) {
                struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
                if (e) {
                    e->timestamp = bpf_ktime_get_ns();
                    e->pid = pid;
                    e->tid = tid & 0xFFFFFFFF;
                    e->event_type = EVENT_TYPE_MMAP;
                    e->addr = (__u64)ret;  /* actual mapped address */
                    e->size = args->len;
                    bpf_ringbuf_submit(e, 0);
                }
            }

            bpf_map_delete_elem(&mmap_temp, &tid);
        }
    }

    return 0;
}

/* munmap tracking */
SEC("tracepoint/syscalls/sys_enter_munmap")
int tracepoint_sys_enter_munmap(struct trace_event_raw_sys_enter *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    if (is_tracked(pid) && is_enabled()) {
        /* munmap(addr, len) */
        __u64 addr = ctx->args[0];
        __u64 len = ctx->args[1];

        /* Report the munmap attempt (we track entry, not exit,
         * because we want to know what was requested even if it fails) */
        if (addr != 0 && len > 0) {
            struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
            if (e) {
                e->timestamp = bpf_ktime_get_ns();
                e->pid = pid;
                e->tid = tid & 0xFFFFFFFF;
                e->event_type = EVENT_TYPE_MUNMAP;
                e->addr = addr;
                e->size = len;
                bpf_ringbuf_submit(e, 0);
            }
        }
    }

    return 0;
}

/* brk tracking - adjusts the program break (heap boundary) */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10000);
    __type(key, __u64);   /* tid */
    __type(value, __u64); /* requested brk value */
} brk_temp SEC(".maps");

SEC("tracepoint/syscalls/sys_enter_brk")
int tracepoint_sys_enter_brk(struct trace_event_raw_sys_enter *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    if (is_tracked(pid)) {
        /* brk(addr) - if addr is 0, just queries current break */
        __u64 requested_brk = ctx->args[0];
        bpf_map_update_elem(&brk_temp, &tid, &requested_brk, BPF_ANY);
    }

    return 0;
}

SEC("tracepoint/syscalls/sys_exit_brk")
int tracepoint_sys_exit_brk(struct trace_event_raw_sys_exit *ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    if (is_tracked(pid) && is_enabled()) {
        __u64 *requested_brk = bpf_map_lookup_elem(&brk_temp, &tid);
        if (requested_brk) {
            __u64 new_brk = ctx->ret;
            __u64 req_brk = *requested_brk;

            /* Only track actual changes (not queries where req == 0) */
            if (req_brk != 0 && new_brk > 0) {
                struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
                if (e) {
                    e->timestamp = bpf_ktime_get_ns();
                    e->pid = pid;
                    e->tid = tid & 0xFFFFFFFF;
                    e->event_type = EVENT_TYPE_BRK;
                    e->addr = new_brk;
                    /* We can't easily determine size change without tracking previous brk,
                     * so we just report the new brk address. Userspace can track deltas. */
                    e->size = 0;
                    bpf_ringbuf_submit(e, 0);
                }
            }

            bpf_map_delete_elem(&brk_temp, &tid);
        }
    }

    return 0;
}
