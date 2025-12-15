// clang-format off
// Prevent clang-format from reformatting the include statement, which is
// needed for the bpf headers below.
#include "vmlinux.h"
// clang-format on

#include <bpf/bpf_core_read.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

#include "event.h"

// Include shared BPF utilities from codspeed-bpf
#include "codspeed/common.h"
#include "codspeed/process_tracking.h"

char LICENSE[] SEC("license") = "GPL";

/* Define process tracking maps using shared macro */
PROCESS_TRACKING_MAPS();

/* Ring buffer for sending events to userspace */
BPF_RINGBUF(events, 256 * 1024);

/* Map to control whether tracking is enabled (0 = disabled, 1 = enabled) */
BPF_ARRAY_MAP(tracking_enabled, __u8, 1);

/* == Code that tracks process forks and execs == */

SEC("tracepoint/sched/sched_process_fork")
int tracepoint_sched_fork(struct trace_event_raw_sched_process_fork* ctx) {
    __u32 parent_pid = ctx->parent_pid;
    __u32 child_pid = ctx->child_pid;

    /* Use shared fork handler */
    handle_fork(parent_pid, child_pid);

    return 0;
}

/* == Helper functions for the allocation tracking == */

/* Helper to check if tracking is currently enabled */
static __always_inline int is_enabled(void) {
    __u32 key = 0;
    __u8* enabled = bpf_map_lookup_elem(&tracking_enabled, &key);

    /* Default to enabled if map not initialized */
    if (!enabled) {
        return 1;
    }

    return *enabled;
}

/* Helper to store parameter value in map for tracking between entry and return
 */
static __always_inline int store_param(void* map, __u64 value) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;
    if (is_tracked(pid)) {
        bpf_map_update_elem(map, &tid, &value, BPF_ANY);
    }
    return 0;
}

/* Helper to take parameter value from map (lookup and delete) */
static __always_inline __u64* take_param(void* map) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u64* value = bpf_map_lookup_elem(map, &tid);
    if (value) {
        bpf_map_delete_elem(map, &tid);
    }
    return value;
}

/* Helper to submit an event to the ring buffer */
static __always_inline int submit_event(__u64 addr, __u64 size, __u8 event_type) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    if (!is_tracked(pid) || !is_enabled()) {
        return 0;
    }

    struct event* e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
    if (!e) {
        return 0;
    }

    e->timestamp = bpf_ktime_get_ns();
    e->pid = pid;
    e->tid = tid & 0xFFFFFFFF;
    e->event_type = event_type;
    e->addr = addr;
    e->size = size;
    bpf_ringbuf_submit(e, 0);

    return 0;
}

/* Macro to generate uprobe/uretprobe pairs for allocation functions */
#define UPROBE_WITH_ARGS(name, size_expr, addr_expr, event_type)                            \
    BPF_HASH_MAP(name##_size, __u64, __u64, 10000);                                         \
    SEC("uprobe")                                                                           \
    int uprobe_##name(struct pt_regs* ctx) { return store_param(&name##_size, size_expr); } \
    SEC("uretprobe")                                                                        \
    int uretprobe_##name(struct pt_regs* ctx) {                                             \
        __u64* size_ptr = take_param(&name##_size);                                         \
        if (!size_ptr) {                                                                    \
            return 0;                                                                       \
        }                                                                                   \
        __u64 addr = addr_expr;                                                             \
        if (addr == 0) {                                                                    \
            return 0;                                                                       \
        }                                                                                   \
        return submit_event(addr, *size_ptr, event_type);                                   \
    }

/* Macro for simple address-only functions like free */
#define UPROBE_ADDR_ONLY(name, addr_expr, event_type) \
    SEC("uprobe")                                     \
    int uprobe_##name(struct pt_regs* ctx) {          \
        __u64 addr = addr_expr;                       \
        if (addr == 0) {                              \
            return 0;                                 \
        }                                             \
        return submit_event(addr, 0, event_type);     \
    }

/* malloc: allocates with size parameter */
UPROBE_WITH_ARGS(malloc, PT_REGS_PARM1(ctx), PT_REGS_RC(ctx), EVENT_TYPE_MALLOC)

/* free: deallocates by address */
UPROBE_ADDR_ONLY(free, PT_REGS_PARM1(ctx), EVENT_TYPE_FREE)

/* calloc: allocates with nmemb * size */
UPROBE_WITH_ARGS(calloc, PT_REGS_PARM1(ctx) * PT_REGS_PARM2(ctx), PT_REGS_RC(ctx), EVENT_TYPE_CALLOC)

/* realloc: reallocates with new size */
UPROBE_WITH_ARGS(realloc, PT_REGS_PARM2(ctx), PT_REGS_RC(ctx), EVENT_TYPE_REALLOC)

/* aligned_alloc: allocates with alignment and size */
UPROBE_WITH_ARGS(aligned_alloc, PT_REGS_PARM2(ctx), PT_REGS_RC(ctx), EVENT_TYPE_ALIGNED_ALLOC)

SEC("tracepoint/syscalls/sys_enter_execve")
int tracepoint_sys_execve(struct trace_event_raw_sys_enter* ctx) { return submit_event(0, 0, EVENT_TYPE_EXECVE); }

/* Map to store mmap parameters between entry and return */
struct mmap_args {
    __u64 addr;
    __u64 len;
};

BPF_HASH_MAP(mmap_temp, __u64, struct mmap_args, 10000);

SEC("tracepoint/syscalls/sys_enter_mmap")
int tracepoint_sys_enter_mmap(struct trace_event_raw_sys_enter* ctx) {
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
int tracepoint_sys_exit_mmap(struct trace_event_raw_sys_exit* ctx) {
    struct mmap_args* args = (struct mmap_args*)take_param(&mmap_temp);
    if (!args) {
        return 0;
    }

    __s64 ret = ctx->ret;
    if (ret <= 0) {
        return 0;
    }

    return submit_event((__u64)ret, args->len, EVENT_TYPE_MMAP);
}

/* munmap tracking */
SEC("tracepoint/syscalls/sys_enter_munmap")
int tracepoint_sys_enter_munmap(struct trace_event_raw_sys_enter* ctx) {
    __u64 addr = ctx->args[0];
    __u64 len = ctx->args[1];

    if (addr == 0 || len == 0) {
        return 0;
    }

    return submit_event(addr, len, EVENT_TYPE_MUNMAP);
}

/* brk tracking - adjusts the program break (heap boundary) */
BPF_HASH_MAP(brk_temp, __u64, __u64, 10000);

SEC("tracepoint/syscalls/sys_enter_brk")
int tracepoint_sys_enter_brk(struct trace_event_raw_sys_enter* ctx) {
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
int tracepoint_sys_exit_brk(struct trace_event_raw_sys_exit* ctx) {
    __u64* requested_brk = take_param(&brk_temp);
    if (!requested_brk) {
        return 0;
    }

    __u64 new_brk = ctx->ret;
    __u64 req_brk = *requested_brk;

    if (req_brk == 0 || new_brk <= 0) {
        return 0;
    }

    return submit_event(new_brk, 0, EVENT_TYPE_BRK);
}
