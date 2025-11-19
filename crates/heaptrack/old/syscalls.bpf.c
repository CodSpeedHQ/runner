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

/* Tracepoint for execve syscall */
SEC("tracepoint/syscalls/sys_enter_execve")
int tracepoint_sys_execve(struct trace_event_raw_sys_enter *ctx) {
    __u32 pid = bpf_get_current_pid_tgid() >> 32;

    /* Check if this process or any parent is being tracked */
    if (is_tracked(pid)) {
        struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
        if (e) {
            e->timestamp = bpf_ktime_get_ns();
            e->pid = pid;
            e->event_type = EVENT_TYPE_EXECVE;
            e->addr = 0;
            e->size = 0;
            bpf_ringbuf_submit(e, 0);
        }
    }

    return 0;
}

/* Temporary storage for mmap parameters between entry and exit */
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

    if (is_tracked(pid)) {
        struct mmap_args *args = bpf_map_lookup_elem(&mmap_temp, &tid);
        if (args) {
            __s64 ret = ctx->ret;

            /* Only track successful mmap calls (ret != MAP_FAILED which is -1) */
            if (ret > 0) {
                struct event *e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
                if (e) {
                    e->timestamp = bpf_ktime_get_ns();
                    e->pid = pid;
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
    __u32 pid = bpf_get_current_pid_tgid() >> 32;

    if (is_tracked(pid)) {
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

    if (is_tracked(pid)) {
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
