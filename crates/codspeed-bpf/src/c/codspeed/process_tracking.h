#ifndef __CODSPEED_PROCESS_TRACKING_H__
#define __CODSPEED_PROCESS_TRACKING_H__

#include "common.h"

/* Standard process tracking maps - define these in your BPF program
 * This macro creates the map definitions. The helper functions below will
 * reference these maps, so call this macro before including this header's helpers.
 */
#define PROCESS_TRACKING_MAPS()                        \
    BPF_HASH_MAP(tracked_pids, __u32, __u8, 10000);   \
    BPF_HASH_MAP(pids_ppid, __u32, __u32, 10000);     \
    /* Helper functions defined below */                                          \
    static __always_inline int is_tracked(__u32 pid);                            \
    static __always_inline int handle_fork(__u32 parent_pid, __u32 child_pid);  \
    static __always_inline int handle_exit(__u32 pid);                           \
    /* is_tracked implementation */                                              \
    static __always_inline int is_tracked(__u32 pid) {                           \
        if (bpf_map_lookup_elem(&tracked_pids, &pid)) {                          \
            return 1;                                                             \
        }                                                                         \
        _Pragma("unroll")                                                        \
        for (int i = 0; i < 5; i++) {                                             \
            __u32* ppid = bpf_map_lookup_elem(&pids_ppid, &pid);                 \
            if (!ppid) {                                                          \
                break;                                                            \
            }                                                                     \
            pid = *ppid;                                                          \
            if (bpf_map_lookup_elem(&tracked_pids, &pid)) {                      \
                return 1;                                                         \
            }                                                                     \
        }                                                                         \
        return 0;                                                                 \
    }                                                                             \
    /* handle_fork implementation */                                             \
    static __always_inline int handle_fork(__u32 parent_pid, __u32 child_pid) { \
        if (is_tracked(parent_pid)) {                                             \
            __u8 marker = 1;                                                      \
            bpf_map_update_elem(&tracked_pids, &child_pid, &marker, BPF_ANY);    \
            bpf_map_update_elem(&pids_ppid, &child_pid, &parent_pid, BPF_ANY);   \
            return 1;                                                             \
        }                                                                         \
        return 0;                                                                 \
    }                                                                             \
    /* handle_exit implementation */                                             \
    static __always_inline int handle_exit(__u32 pid) {                          \
        bpf_map_delete_elem(&tracked_pids, &pid);                                \
        bpf_map_delete_elem(&pids_ppid, &pid);                                   \
        return 0;                                                                 \
    }

#endif /* __CODSPEED_PROCESS_TRACKING_H__ */
