#ifndef VALGRIND_WRAPPER_H
#define VALGRIND_WRAPPER_H

int running_on_valgrind();
void callgrind_dump_stats_at(const char *metadata);
void callgrind_zero_stats();
void callgrind_start_instrumentation();
void callgrind_stop_instrumentation();

#endif
