const bincode = @import("bincode.zig");
const std = @import("std");
const shared = @import("shared.zig");

const fs = std.fs;
const os = std.os;
const mem = std.mem;
const Allocator = std.mem.Allocator;
const Path = []const u8;
pub const Command = shared.Command;

extern "c" fn mkfifo(path: [*:0]const u8, mode: c_uint) c_int;

pub const UnixPipe = struct {
    pub const Reader = struct {
        file: fs.File,
        allocator: Allocator,

        pub fn init(file: fs.File, allocator: Allocator) Reader {
            return .{
                .file = file,
                .allocator = allocator,
            };
        }

        pub fn read(self: *Reader, buffer: []u8) !usize {
            return self.file.read(buffer);
        }

        pub fn readAll(self: *Reader, buffer: []u8) !usize {
            return self.file.readAll(buffer);
        }

        pub fn recvCmd(self: *Reader) !Command {
            // First read the length (u32 = 4 bytes)
            var len_buffer: [4]u8 = undefined;
            _ = try self.file.readAll(&len_buffer);
            const message_len = std.mem.readInt(u32, &len_buffer, std.builtin.Endian.little);

            // Read the message
            const buffer = try self.allocator.alloc(u8, message_len);
            defer self.allocator.free(buffer);

            while (true) {
                _ = self.file.readAll(buffer) catch {
                    continue;
                };
                break;
            }

            var stream = std.io.fixedBufferStream(buffer);
            return try bincode.deserializeAlloc(stream.reader(), self.allocator, Command);
        }

        pub fn waitForAck(self: *Reader) !void {
            while (true) {
                const cmd = self.recvCmd() catch {
                    continue;
                };
                if (cmd == Command.Ack) {
                    break;
                }
            }
        }

        pub fn deinit(self: *Reader) void {
            self.file.close();
        }
    };

    pub const Writer = struct {
        file: fs.File,
        allocator: Allocator,

        pub fn init(file: fs.File, allocator: Allocator) Writer {
            return .{
                .file = file,
                .allocator = allocator,
            };
        }

        pub fn write(self: *Writer, buffer: []const u8) !usize {
            return self.file.write(buffer);
        }

        pub fn writeAll(self: *Writer, buffer: []const u8) !void {
            return self.file.writeAll(buffer);
        }

        pub fn sendCmd(self: *Writer, cmd: Command) !void {
            var buffer = std.ArrayList(u8).init(self.allocator);
            defer buffer.deinit();

            try bincode.serialize(buffer.writer(), cmd);

            const bytes = buffer.items;
            try self.file.writeAll(std.mem.asBytes(&@as(u32, @intCast(bytes.len))));
            try self.file.writeAll(bytes);
        }

        pub fn deinit(self: *Writer) void {
            self.file.close();
        }
    };

    /// Create a new named pipe at the given path
    pub fn create(allocator: Allocator, path: [*:0]const u8) !void {
        _ = allocator;

        // Remove the previous FIFO (if it exists)
        fs.deleteFileAbsolute(std.mem.span(path)) catch {};

        if (mkfifo(path, 0o700) != 0) {
            return error.FifoCreationFailed;
        }
    }

    pub fn openRead(allocator: Allocator, path: []const u8) !Reader {
        const file = try fs.openFileAbsolute(path, .{ .mode = .read_write, .lock = .none, .lock_nonblocking = true });
        return Reader.init(file, allocator);
    }

    pub fn openWrite(allocator: Allocator, path: []const u8) !Writer {
        const file = try fs.openFileAbsolute(path, .{ .mode = .write_only, .lock = .none, .lock_nonblocking = true });
        return Writer.init(file, allocator);
    }
};

pub const BenchGuard = struct {
    ctl_writer: UnixPipe.Writer,
    ack_reader: UnixPipe.Reader,
    allocator: Allocator,

    pub fn init(allocator: Allocator, ctl_fifo_path: []const u8, ack_fifo_path: []const u8) !*BenchGuard {
        var self = try allocator.create(BenchGuard);
        errdefer allocator.destroy(self);

        self.allocator = allocator;
        self.ctl_writer = try UnixPipe.openWrite(allocator, ctl_fifo_path);
        self.ack_reader = try UnixPipe.openRead(allocator, ack_fifo_path);

        try self.sendCmd(Command.StartBenchmark);
        return self;
    }

    pub fn initWithRunnerFifo(allocator: Allocator) !*BenchGuard {
        return try BenchGuard.init(allocator, shared.RUNNER_CTL_FIFO, shared.RUNNER_ACK_FIFO);
    }

    pub fn deinit(self: *BenchGuard) void {
        self.sendCmd(Command.StopBenchmark) catch {};
        self.ctl_writer.deinit();
        self.ack_reader.deinit();
        self.allocator.destroy(self);
    }

    fn sendCmd(self: *BenchGuard, cmd: Command) !void {
        try self.ctl_writer.sendCmd(cmd);
        try self.ack_reader.waitForAck();
    }
};

pub fn sendCmd(allocator: Allocator, cmd: Command) !void {
    var writer = try UnixPipe.openWrite(allocator, shared.RUNNER_CTL_FIFO);
    defer writer.deinit();
    try writer.sendCmd(cmd);

    var reader = try UnixPipe.openRead(allocator, shared.RUNNER_ACK_FIFO);
    defer reader.deinit();
    try reader.waitForAck();
}

test "fail if doesn't exist" {
    const allocator = std.testing.allocator;

    const nonexistent_path = "/tmp/nonexistent_pipe_test.fifo";

    // Ensure it doesn't exist
    fs.deleteFileAbsolute(nonexistent_path) catch {};

    // Attempt to open for reading should fail
    const reader_result = UnixPipe.openRead(allocator, nonexistent_path);
    try std.testing.expectError(error.FileNotFound, reader_result);

    // Attempt to open for writing should fail
    const writer_result = UnixPipe.openWrite(allocator, nonexistent_path);
    try std.testing.expectError(error.FileNotFound, writer_result);

    // Attempt to send cmd to runner fifo
    fs.deleteFileAbsolute(shared.RUNNER_ACK_FIFO) catch {};
    fs.deleteFileAbsolute(shared.RUNNER_CTL_FIFO) catch {};

    const sendcmd_result = sendCmd(allocator, Command.StartBenchmark);
    try std.testing.expectError(error.FileNotFound, sendcmd_result);
}

test "unix pipe write read" {
    const allocator = std.testing.allocator;
    const test_path = "/tmp/test1.fifo";

    try UnixPipe.create(allocator, test_path);

    var reader = try UnixPipe.openRead(allocator, test_path);
    defer reader.deinit();

    var writer = try UnixPipe.openWrite(allocator, test_path);
    defer writer.deinit();

    const message = "Hello";
    try writer.writeAll(message);

    var buffer: [5]u8 = undefined;
    _ = try reader.readAll(&buffer);

    try std.testing.expectEqualStrings(message, &buffer);
}

test "unix pipe send recv cmd" {
    const allocator = std.testing.allocator;
    const test_path = "/tmp/test2.fifo";

    try UnixPipe.create(allocator, test_path);

    var reader = try UnixPipe.openRead(allocator, test_path);
    defer reader.deinit();

    var writer = try UnixPipe.openWrite(allocator, test_path);
    defer writer.deinit();

    try writer.sendCmd(Command.StartBenchmark);
    const cmd = try reader.recvCmd();

    try std.testing.expectEqual(Command.StartBenchmark, cmd);
}
