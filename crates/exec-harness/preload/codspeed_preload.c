// LD_PRELOAD library for enabling Valgrind instrumentation in child processes
//
// This library is loaded via LD_PRELOAD into benchmark processes spawned by
// exec-harness. It enables callgrind instrumentation on load and disables it on
// exit, allowing exec-harness to measure arbitrary commands without requiring
// them to link against instrument-hooks.
//
// Environment variables:
//   CODSPEED_BENCH_URI - The benchmark URI to report (required)
//   CODSPEED_PRELOAD_LOCK - Set by the first process to prevent child processes
//                          from re-initializing instrumentation

#include <stdlib.h>
#include <unistd.h>

#include "core.h"

#ifndef RUNNING_ON_VALGRIND
// If somehow the core.h did not include the valgrind header, something is
// wrong, but still have a fallback
#warning "RUNNING_ON_VALGRIND not defined, headers may be missing"
#define RUNNING_ON_VALGRIND 0
#endif

static const char *LOCK_ENV = "CODSPEED_PRELOAD_LOCK";

// These constants are defined by the build script (build.rs) via -D flags
#ifndef CODSPEED_URI_ENV
#error "CODSPEED_URI_ENV must be defined by the build system"
#endif
#ifndef CODSPEED_INTEGRATION_NAME
#error "CODSPEED_INTEGRATION_NAME must be defined by the build system"
#endif
#ifndef CODSPEED_INTEGRATION_VERSION
#error "CODSPEED_INTEGRATION_VERSION must be defined by the build system"
#endif

static const char *URI_ENV = CODSPEED_URI_ENV;
static const char *INTEGRATION_NAME = CODSPEED_INTEGRATION_NAME;
static const char *INTEGRATION_VERSION = CODSPEED_INTEGRATION_VERSION;

static InstrumentHooks *g_hooks = NULL;
static const char *g_bench_uri = NULL;

__attribute__((constructor)) static void codspeed_preload_init(void) {
  // Skip initialization if not running under Valgrind yet.
  // When using LD_PRELOAD with Valgrind, the constructor runs twice:
  // once before Valgrind takes over, and once after. We only want to
  // initialize when Valgrind is active.
  //
  // This is purely empirical, and is not (yet) backed up by documented
  // behavior.
  if (!RUNNING_ON_VALGRIND) {
    return;
  }

  // Check if another process already owns the instrumentation
  if (getenv(LOCK_ENV)) {
    return;
  }

  // Set the lock to prevent child processes from initializing
  setenv(LOCK_ENV, "1", 1);

  g_bench_uri = getenv(URI_ENV);
  if (!g_bench_uri) {
    return;
  }

  g_hooks = instrument_hooks_init();
  if (!g_hooks) {
    return;
  }

  instrument_hooks_set_integration(g_hooks, INTEGRATION_NAME,
                                   INTEGRATION_VERSION);

  if (instrument_hooks_start_benchmark_inline(g_hooks) != 0) {
    instrument_hooks_deinit(g_hooks);
    g_hooks = NULL;
    return;
  }
}

__attribute__((destructor)) static void codspeed_preload_fini(void) {
  // If the process is not the owner of the lock, this means g_hooks was not
  // initialized
  if (!g_hooks) {
    return;
  }

  instrument_hooks_stop_benchmark_inline(g_hooks);

  int32_t pid = getpid();
  instrument_hooks_set_executed_benchmark(g_hooks, pid, g_bench_uri);

  instrument_hooks_deinit(g_hooks);
  g_hooks = NULL;
}
