const std = @import("std");

// !!!!!!!!!!!!!!!!!!!!!!!!
// !! DO NOT TOUCH BELOW !!
// !!!!!!!!!!!!!!!!!!!!!!!!
//
pub const RUNNER_CTL_FIFO = "/tmp/runner.ctl.fifo";
pub const RUNNER_ACK_FIFO = "/tmp/runner.ack.fifo";

pub const Command = union(enum) {
    CurrentBenchmark: struct {
        pid: u32,
        uri: []const u8,
    },
    StartBenchmark,
    StopBenchmark,
    Ack,

    pub fn format(
        self: Command,
        comptime fmt: []const u8,
        options: std.fmt.FormatOptions,
        writer: anytype,
    ) !void {
        _ = fmt;
        _ = options;
        switch (self) {
            .CurrentBenchmark => |data| try writer.print("CurrentBenchmark {{ pid: {d}, uri: {s} }}", .{ data.pid, data.uri }),
            .StartBenchmark => try writer.writeAll("StartBenchmark"),
            .StopBenchmark => try writer.writeAll("StopBenchmark"),
            .Ack => try writer.writeAll("Ack"),
        }
    }
};
//
// !!!!!!!!!!!!!!!!!!!!!!!!
// !! DO NOT TOUCH ABOVE !!
// !!!!!!!!!!!!!!!!!!!!!!!!
