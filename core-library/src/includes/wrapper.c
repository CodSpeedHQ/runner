// Small wrapper that exposes the macros that can't be parsed by Zig

#include "callgrind.h"
#include "valgrind.h"

int running_on_valgrind() { return RUNNING_ON_VALGRIND; }

void callgrind_dump_stats_at(const char *metadata) {
  CALLGRIND_DUMP_STATS_AT(metadata);
}

void callgrind_zero_stats() {
    CALLGRIND_ZERO_STATS;
}

void callgrind_start_instrumentation() {
    CALLGRIND_START_INSTRUMENTATION;
}

void callgrind_stop_instrumentation() {
    CALLGRIND_STOP_INSTRUMENTATION;
}
