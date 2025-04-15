const std = @import("std");
const builtin = @import("builtin");

pub fn main() !void {
    const out = std.io.getStdOut().writer();
    for (builtin.test_functions) |t| {
        const result = t.func();
        const name = extractName(t);
        if (result) |_| {
            try std.fmt.format(out, "[SUCCESS] {s}\n", .{name});
        } else |err| {
            try std.fmt.format(out, "[FAIL] {s}: {}\n", .{t.name, err});
        }
    }
}

fn extractName(t: std.builtin.TestFn) []const u8 {
    const marker = std.mem.lastIndexOf(u8, t.name, ".test.") orelse return t.name;
    return t.name[marker+6..];
}
