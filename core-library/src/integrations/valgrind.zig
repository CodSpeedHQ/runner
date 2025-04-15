const std = @import("std");
const valgrind = @cImport({
    @cInclude("wrapper.h");
});

pub fn is_running() bool {
    return valgrind.running_on_valgrind() != 0;
}

pub fn set_integration(allocator: std.mem.Allocator, name: [*c]const u8, version: [*c]const u8) !void {
    const metadata = try std.fmt.allocPrint(
        allocator,
        "Metadata: {s} {s}",
        .{ name, version },
    );

    valgrind.callgrind_dump_stats_at(metadata.ptr);
}

pub fn start_benchmark() void {
    valgrind.callgrind_zero_stats();
    valgrind.callgrind_start_instrumentation();
}

pub fn stop_benchmark() void {
    valgrind.callgrind_stop_instrumentation();
}

pub fn current_benchmark(pid: u32, uri: [*c]const u8) void {
    _ = pid;

    valgrind.callgrind_dump_stats_at(uri);
}
