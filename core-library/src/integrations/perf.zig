const std = @import("std");
const fifo = @import("../fifo.zig");

pub fn is_running() bool {
    // fifo.sendCmd(std.heap.c_allocator, fifo.Command.Ping)
    return false;
}

pub fn set_integration(name: [*c]const u8, version: [*c]const u8) !void {
    _ = name;
    _ = version;

    // TODO: Handle this
}

pub fn start_benchmark(allocator: std.mem.Allocator) !void {
    try fifo.sendCmd(allocator, fifo.Command.StartBenchmark);
}

pub fn stop_benchmark(allocator: std.mem.Allocator) !void {
    try fifo.sendCmd(allocator, fifo.Command.StopBenchmark);
}

pub fn current_benchmark(allocator: std.mem.Allocator, pid: u32, uri: [*c]const u8) !void {
    const uri_str = std.mem.span(uri);
    std.debug.print("PID: {}, URI: {s}\n", .{ pid, uri_str });

    try fifo.sendCmd(allocator, fifo.Command{ .CurrentBenchmark = .{
        .pid = pid,
        .uri = uri_str,
    }});
}
