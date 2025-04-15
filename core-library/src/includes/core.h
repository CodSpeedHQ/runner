#ifndef CORE_H
#define CORE_H

#include <stdint.h>
#include <stdbool.h>

bool is_instrumented(void);
void start_benchmark(void);
void stop_benchmark(void);
void current_benchmark(int32_t pid, const char* uri);
void set_integration(const char *name, const char* version);

#endif
