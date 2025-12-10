// clang-format off
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

// Define standard process tracking maps using shared macro
PROCESS_TRACKING_MAPS();

// Define ring buffer for events
BPF_RINGBUF(events, 256 * 1024);

/* Helper to submit an event to the ring buffer */
static __always_inline int submit_event(__u8 event_type, __u32 pid, __u32 ppid) {
    if (!is_tracked(pid) && !is_tracked(ppid)) {
        return 0;
    }

    struct event* e = bpf_ringbuf_reserve(&events, sizeof(*e), 0);
    if (!e) {
        return 0;
    }

    __u64 tid_full = bpf_get_current_pid_tgid();

    e->timestamp = bpf_ktime_get_ns();
    e->pid = pid;
    e->tid = tid_full & 0xFFFFFFFF;
    e->ppid = ppid;
    e->event_type = event_type;

    // Get current command name
    bpf_get_current_comm(e->comm, sizeof(e->comm));

    bpf_ringbuf_submit(e, 0);
    return 0;
}

/* Track process creation via fork/clone */
SEC("tracepoint/sched/sched_process_fork")
int tracepoint_sched_fork(struct trace_event_raw_sched_process_fork* ctx) {
    __u32 parent_pid = ctx->parent_pid;
    __u32 child_pid = ctx->child_pid;

    // Use shared fork handler to track child
    if (handle_fork(parent_pid, child_pid)) {
        // Submit fork event with parent/child relationship
        submit_event(EVENT_TYPE_FORK, child_pid, parent_pid);
    }

    return 0;
}

/* Track process execution via execve */
SEC("tracepoint/sched/sched_process_exec")
int tracepoint_sched_exec(struct trace_event_raw_sched_process_exec* ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    if (is_tracked(pid)) {
        submit_event(EVENT_TYPE_EXEC, pid, 0);
    }

    return 0;
}

/* Track process termination via exit */
SEC("tracepoint/sched/sched_process_exit")
int tracepoint_sched_exit(struct trace_event_raw_sched_process_template* ctx) {
    __u64 tid = bpf_get_current_pid_tgid();
    __u32 pid = tid >> 32;

    if (is_tracked(pid)) {
        submit_event(EVENT_TYPE_EXIT, pid, 0);
        // Use shared exit handler to clean up maps
        handle_exit(pid);
    }

    return 0;
}
