const perf = @import("integrations/perf.zig");
const valgrind = @import("integrations/valgrind.zig");

const fifo = @import("fifo.zig");
const std = @import("std");

pub export fn start_benchmark() void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();

    if (valgrind.is_running()) {
        valgrind.start_benchmark();
    }

    if (perf.is_running()) {
        perf.start_benchmark(gpa.allocator()) catch {
            std.debug.print("Error starting benchmark\n", .{});
        };
    }
}

pub export fn stop_benchmark() void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();

    if (valgrind.is_running()) {
        valgrind.stop_benchmark();
    }

    if (perf.is_running()) {
        perf.stop_benchmark(gpa.allocator()) catch {
            std.debug.print("Error stopping benchmark\n", .{});
        };
    }
}

pub export fn current_benchmark(pid: u32, uri: [*c]const u8) void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();

    if (valgrind.is_running()) {
        valgrind.current_benchmark(pid, uri);
    }

    if (perf.is_running()) {
        perf.current_benchmark(gpa.allocator(), pid, uri) catch {
            std.debug.print("Error setting current benchmark\n", .{});
        };
    }
}

pub export fn set_integration(name: [*c]const u8, version: [*c]const u8) void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();

    if (valgrind.is_running()) {
        valgrind.set_integration(gpa.allocator(), name, version) catch {
            std.debug.print("Failed to set integration", .{});
        };
    }

    if (perf.is_running()) {
        perf.set_integration(name, version) catch {
            std.debug.print("Failed to set integration", .{});
        };
    }
}

test {
    _ = @import("bincode.zig");
    _ = @import("fifo.zig");
}

test "no crash when not instrumented" {
    _ = start_benchmark();
    _ = stop_benchmark();
    _ = current_benchmark(0, "test");
    _ = set_integration("test", "1.0");

}
