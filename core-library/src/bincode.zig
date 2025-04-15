// Taken from here: https://github.com/qbradley/bincode-zig/blob/main/bincode.zig

const std = @import("std");

pub fn deserializeAlloc(stream: anytype, allocator: std.mem.Allocator, comptime T: type) !T {
    return switch (@typeInfo(T)) {
        .Void => {},
        .Bool => try deserializeBool(stream),
        .Float => try deserializeFloat(stream, T),
        .Int => try deserializeInt(stream, T),
        .Optional => |info| try deserializeOptionalAlloc(stream, allocator, info.child),
        .Pointer => |info| try deserializePointerAlloc(stream, info, allocator),
        .Array => |info| try deserializeArrayAlloc(stream, info, allocator),
        .Struct => |info| try deserializeStructAlloc(stream, info, allocator, T),
        .Enum => try deserializeEnum(stream, T),
        .Union => |info| try deserializeUnionAlloc(stream, info, allocator, T),
        else => unsupportedType(T),
    };
}

pub fn deserialize(stream: anytype, comptime T: type) !T {
    return switch (@typeInfo(T)) {
        .Void => {},
        .Bool => try deserializeBool(stream),
        .Float => try deserializeFloat(stream, T),
        .Int => try deserializeInt(stream, T),
        .Optional => |info| try deserializeOptional(stream, info.child),
        .Array => |info| try deserializeArray(stream, info),
        .Struct => |info| try deserializeStruct(stream, info, T),
        .Enum => try deserializeEnum(stream, T),
        .Union => |info| try deserializeUnion(stream, info, T),
        else => unsupportedType(T),
    };
}

pub fn deserializeBuffer(comptime T: type, source: *[]const u8) T {
    return switch (@typeInfo(T)) {
        .Void => {},
        .Bool => deserializeBufferBool(source),
        .Float => deserializeBufferFloat(T, source),
        .Int => deserializeBufferInt(T, source),
        .Optional => |info| deserializeBufferOptional(info.child, source),
        .Pointer => |info| deserializeBufferPointer(info, source),
        .Array => |info| deserializeBufferArray(info, source),
        .Struct => |info| deserializeBufferStruct(T, info, source),
        .Enum => deserializeBufferEnum(T, source),
        .Union => |info| deserializeBufferUnion(T, info, source),
        else => unsupportedType(T),
    };
}

pub fn serialize(stream: anytype, value: anytype) @TypeOf(stream).Error!void {
    const T = @TypeOf(value);
    return switch (@typeInfo(T)) {
        .Void => {},
        .Bool => try serializeBool(stream, value),
        .Float => try serializeFloat(stream, T, value),
        .Int => try serializeInt(stream, T, value),
        .Optional => |info| try serializeOptional(stream, info.child, value),
        .Pointer => |info| try serializePointer(stream, info, T, value),
        .Array => |info| try serializeArray(stream, info, T, value),
        .Struct => |info| try serializeStruct(stream, info, T, value),
        .Enum => try serializeEnum(stream, T, value),
        .Union => |info| try serializeUnion(stream, info, T, value),
        else => unsupportedType(T),
    };
}

pub fn deserializeSliceIterator(comptime T: type, source: []const u8) DeserializeSliceIterator(T) {
    return DeserializeSliceIterator(T){
        .source = source,
    };
}

pub fn DeserializeSliceIterator(comptime T: type) type {
    return struct {
        source: []const u8,

        pub fn next(self: *@This()) ?T {
            if (self.source.len > 0) {
                return deserializeBuffer(T, &self.source);
            } else {
                return null;
            }
        }
    };
}

fn deserializeBufferInt(comptime T: type, source_ptr: *[]const u8) T {
    const bytesRequired = @sizeOf(T);
    const source = source_ptr.*;
    if (bytesRequired <= source.len) {
        var tmp: [bytesRequired]u8 = undefined;
        std.mem.copy(u8, &tmp, source[0..bytesRequired]);
        source_ptr.* = source[bytesRequired..];
        return std.mem.readIntLittle(T, &tmp);
    } else {
        invalidProtocol("Buffer ran out of bytes too soon.");
    }
}

fn deserializeBufferBool(source: *[]const u8) bool {
    return switch (deserializeBufferInt(u8, source)) {
        0 => return false,
        1 => return true,
        else => invalidProtocol("Boolean values should be encoded as a single byte with value 0 or 1 only."),
    };
}

fn deserializeBufferOptional(comptime T: type, source: *[]const u8) ?T {
    if (deserializeBufferBool(source)) {
        return deserializeBuffer(T, source);
    } else {
        return null;
    }
}

fn deserializeBufferFloat(comptime T: type, source: *[]const u8) T {
    switch (T) {
        f32 => return @as(T, @bitCast(deserializeBufferInt(u32, source))),
        f64 => return @as(T, @bitCast(deserializeBufferInt(u64, source))),
        else => unsupportedType(T),
    }
}

fn deserializeBufferEnum(comptime T: type, source: *[]const u8) T {
    const raw_tag = deserializeBufferInt(u32, source);
    return @enumFromInt(raw_tag);
}

fn deserializeBufferStruct(comptime T: type, comptime info: std.builtin.Type.Struct, source: *[]const u8) T {
    var value: T = undefined;
    inline for (info.fields) |field| {
        @field(value, field.name) = deserializeBuffer(field.type, source);
    }
    return value;
}

fn deserializeBufferUnion(comptime T: type, comptime info: std.builtin.Type.Union, source: *[]const u8) T {
    if (info.tag_type) |Tag| {
        const raw_tag = deserializeBufferInt(u32, source);
        const tag = @as(T, @enumFromInt(raw_tag));

        inline for (info.fields) |field| {
            if (tag == @field(Tag, field.name)) {
                const inner = deserializeBuffer(field.type, source);
                return @unionInit(T, field.name, inner);
            }
        }
        unreachable;
    } else {
        unsupportedType(T);
    }
}

fn deserializeBufferArray(comptime info: std.builtin.Type.Array, source_ptr: *[]const u8) [info.len]info.child {
    const T = @Type(.{ .Array = info });
    if (info.sentinel != null) unsupportedType(T);
    var value: T = undefined;
    if (info.child == u8) {
        const source = source_ptr.*;
        if (info.len <= source.len) {
            std.mem.copy(u8, &value, source[0..info.len]);
            source_ptr.* = source[info.len..];
        } else {
            invalidProtocol("The stream end was found before all required bytes were read.");
        }
    } else {
        for (0..info.len) |idx| {
            value[idx] = deserializeBuffer(info.child, source_ptr);
        }
    }
    return value;
}

fn deserializeBufferPointer(comptime info: std.builtin.Type.Pointer, source_ptr: *[]const u8) []const info.child {
    const T = @Type(.{ .Pointer = info });
    if (info.sentinel != null) unsupportedType(T);
    switch (info.size) {
        .One => unsupportedType(T),
        .Slice => {
            const len = @as(usize, @intCast(deserializeBufferInt(u64, source_ptr)));
            if (info.child == u8) {
                const source = source_ptr.*;
                if (len <= source.len) {
                    source_ptr.* = source[len..];
                    return source[0..len];
                } else {
                    invalidProtocol("The stream end was found before all required bytes were read.");
                }
            } else {
                // we can't support a variable slice of types where the stream format
                // differs from in-memory format without allocating.
                unsupportedType(T);
            }
        },
        .C => unsupportedType(T),
        .Many => unsupportedType(T),
    }
}

fn deserializeBool(stream: anytype) !bool {
    switch (try stream.readInt(u8, .little)) {
        0 => return false,
        1 => return true,
        else => invalidProtocol("Boolean values should be encoded as a single byte with value 0 or 1 only."),
    }
}

fn deserializeFloat(stream: anytype, comptime T: type) !T {
    switch (T) {
        f32 => return @as(T, @bitCast(try stream.readInt(u32, .little))),
        f64 => return @as(T, @bitCast(try stream.readInt(u64, .little))),
        else => unsupportedType(T),
    }
}

fn deserializeInt(stream: anytype, comptime T: type) !T {
    switch (T) {
        i8 => return try stream.readInt(i8, .little),
        i16 => return try stream.readInt(i16, .little),
        i32 => return try stream.readInt(i32, .little),
        i64 => return try stream.readInt(i64, .little),
        i128 => return try stream.readInt(i128, .little),
        u8 => return try stream.readInt(u8, .little),
        u16 => return try stream.readInt(u16, .little),
        u32 => return try stream.readInt(u32, .little),
        u64 => return try stream.readInt(u64, .little),
        u128 => return try stream.readInt(u128, .little),
        else => unsupportedType(T),
    }
}

fn deserializeOptionalAlloc(stream: anytype, allocator: std.mem.Allocator, comptime T: type) !?T {
    switch (try stream.readInt(u8, .little)) {
        // None
        0 => return null,
        // Some
        1 => return try deserializeAlloc(stream, allocator, T),
        else => invalidProtocol("Optional is encoded as a single 0 valued byte for null, or a single 1 valued byte followed by the encoding of the contained value."),
    }
}

fn deserializeOptional(stream: anytype, comptime T: type) !?T {
    switch (try stream.readInt(u8, .little)) {
        // None
        0 => return null,
        // Some
        1 => return try deserialize(stream, T),
        else => invalidProtocol("Optional is encoded as a single 0 valued byte for null, or a single 1 valued byte followed by the encoding of the contained value."),
    }
}

fn deserializePointerAlloc(stream: anytype, comptime info: std.builtin.Type.Pointer, allocator: std.mem.Allocator) ![]info.child {
    const T = @Type(.{ .Pointer = info });
    if (info.sentinel != null) unsupportedType(T);
    switch (info.size) {
        .One => unsupportedType(T),
        .Slice => {
            const len = @as(usize, @intCast(try stream.readInt(u64, .little)));
            var memory = try allocator.alloc(info.child, len);
            if (info.child == u8) {
                const amount = try stream.readAll(memory);
                if (amount != len) {
                    invalidProtocol("The stream end was found before all required bytes were read.");
                }
            } else {
                for (0..len) |idx| {
                    memory[idx] = try deserializeAlloc(stream, allocator, info.child);
                }
            }
            return memory;
        },
        .C => unsupportedType(T),
        .Many => unsupportedType(T),
    }
}

fn deserializeArrayAlloc(stream: anytype, comptime info: std.builtin.Type.Array, allocator: std.mem.Allocator) ![info.len]info.child {
    const T = @Type(.{ .Array = info });
    if (info.sentinel != null) unsupportedType(T);
    var value: T = undefined;
    if (info.child == u8) {
        const amount = try stream.readAll(value[0..]);
        if (amount != info.len) {
            invalidProtocol("The stream end was found before all required bytes were read.");
        }
    } else {
        for (0..info.len) |idx| {
            value[idx] = try deserializeAlloc(stream, allocator, info.child);
        }
    }
    return value;
}

fn deserializeArray(stream: anytype, comptime info: std.builtin.Type.Array) ![info.len]info.child {
    const T = @Type(.{ .Array = info });
    if (info.sentinel != null) unsupportedType(T);
    var value: T = undefined;
    if (info.child == u8) {
        const amount = try stream.readAll(value[0..]);
        if (amount != info.len) {
            invalidProtocol("The stream end was found before all required bytes were read.");
        }
    } else {
        for (0..info.len) |idx| {
            value[idx] = try deserialize(stream, info.child);
        }
    }
    return value;
}

fn deserializeStructAlloc(stream: anytype, comptime info: std.builtin.Type.Struct, allocator: std.mem.Allocator, comptime T: type) !T {
    var value: T = undefined;
    inline for (info.fields) |field| {
        @field(value, field.name) = try deserializeAlloc(stream, allocator, field.type);
    }
    return value;
}

fn deserializeStruct(stream: anytype, comptime info: std.builtin.Type.Struct, comptime T: type) !T {
    var value: T = undefined;
    inline for (info.fields) |field| {
        @field(value, field.name) = try deserialize(stream, field.type);
    }
    return value;
}

fn deserializeEnum(stream: anytype, comptime T: type) !T {
    const raw_tag = try deserializeInt(stream, u32);
    return @as(T, @enumFromInt(raw_tag));
}

fn deserializeUnionAlloc(stream: anytype, comptime info: std.builtin.Type.Union, allocator: std.mem.Allocator, comptime T: type) !T {
    if (info.tag_type) |Tag| {
        const raw_tag = try deserializeAlloc(stream, allocator, u32);
        const tag = @as(Tag, @enumFromInt(raw_tag));

        inline for (info.fields) |field| {
            if (tag == @field(Tag, field.name)) {
                const inner = try deserializeAlloc(stream, allocator, field.type);
                return @unionInit(T, field.name, inner);
            }
        }
        unreachable;
    } else {
        unsupportedType(T);
    }
}

fn deserializeUnion(stream: anytype, comptime info: std.builtin.Type.Union, comptime T: type) !T {
    if (info.tag_type) |Tag| {
        const raw_tag = try deserialize(stream, u32);
        const tag = @as(T, @enumFromInt(raw_tag));

        inline for (info.fields) |field| {
            if (tag == @field(Tag, field.name)) {
                const inner = try deserialize(stream, field.type);
                return @unionInit(T, field.name, inner);
            }
        }
        unreachable;
    } else {
        unsupportedType(T);
    }
}

pub fn serializeBool(stream: anytype, value: bool) @TypeOf(stream).Error!void {
    const code: u8 = if (value) @as(u8, 1) else @as(u8, 0);
    return stream.writeIntLittle(u8, code);
}

pub fn serializeFloat(stream: anytype, comptime T: type, value: T) @TypeOf(stream).Error!void {
    switch (T) {
        f32 => try stream.writeIntLittle(u32, @as(u32, @bitCast(value))),
        f64 => try stream.writeIntLittle(u64, @as(u64, @bitCast(value))),
        else => unsupportedType(T),
    }
}

pub fn serializeInt(stream: anytype, comptime T: type, value: T) @TypeOf(stream).Error!void {
    switch (T) {
        i8 => try stream.writeInt(i8, value, .little),
        i16 => try stream.writeInt(i16, value, .little),
        i32 => try stream.writeInt(i32, value, .little),
        i64 => try stream.writeInt(i64, value, .little),
        i128 => try stream.writeInt(i128, value, .little),
        u8 => try stream.writeInt(u8, value, .little),
        u16 => try stream.writeInt(u16, value, .little),
        u32 => try stream.writeInt(u32, value, .little),
        u64 => try stream.writeInt(u64, value, .little),
        u128 => try stream.writeInt(u128, value, .little),
        else => unsupportedType(T),
    }
}

pub fn serializeOptional(stream: anytype, comptime T: type, value: ?T) @TypeOf(stream).Error!void {
    if (value) |actual| {
        try stream.writeInt(u8, 1, .little);
        try serialize(stream, actual);
    } else {
        // None
        try stream.writeInt(u8, 0, .little);
    }
}

pub fn serializePointer(stream: anytype, comptime info: std.builtin.Type.Pointer, comptime T: type, value: T) @TypeOf(stream).Error!void {
    if (info.sentinel != null) unsupportedType(T);
    switch (info.size) {
        .One => unsupportedType(T),
        .Slice => {
            try stream.writeInt(u64, value.len, .little);
            if (info.child == u8) {
                try stream.writeAll(value);
            } else {
                for (value) |item| {
                    try serialize(stream, item);
                }
            }
        },
        .C => unsupportedType(T),
        .Many => unsupportedType(T),
    }
}

pub fn serializeArray(stream: anytype, comptime info: std.builtin.Type.Array, comptime T: type, value: T) @TypeOf(stream).Error!void {
    if (info.sentinel != null) unsupportedType(T);
    if (info.child == u8) {
        try stream.writeAll(value);
    } else {
        for (value) |item| {
            try serialize(stream, item);
        }
    }
}

pub fn serializeStruct(stream: anytype, comptime info: std.builtin.Type.Struct, comptime T: type, value: T) @TypeOf(stream).Error!void {
    inline for (info.fields) |field| {
        try serialize(stream, @field(value, field.name));
    }
}

pub fn serializeEnum(stream: anytype, comptime T: type, value: T) @TypeOf(stream).Error!void {
    const tag: u32 = @intFromEnum(value);
    try serialize(stream, tag);
}

pub fn serializeUnion(stream: anytype, comptime info: std.builtin.Type.Union, comptime T: type, value: T) @TypeOf(stream).Error!void {
    if (info.tag_type) |UnionTagType| {
        const tag: u32 = @intFromEnum(value);
        try serialize(stream, tag);
        inline for (info.fields) |field| {
            if (value == @field(UnionTagType, field.name)) {
                try serialize(stream, @field(value, field.name));
            }
        }
    } else {
        unsupportedType(T);
    }
}

fn unsupportedType(comptime T: type) noreturn {
    @compileError("Unsupported type " ++ @typeName(T));
}

fn invalidProtocol(comptime message: []const u8) noreturn {
    @panic("Invalid protocol detected: " ++ message);
}

test "example" {
    const bincode = @This(); //@import("bincode-zig");

    const Shared = struct {
        name: []const u8,
        age: u32,
    };

    const example = Shared{ .name = "Cat", .age = 5 };

    // Serialize Shared to buffer
    var buffer: [8192]u8 = undefined;
    var output_stream = std.io.fixedBufferStream(buffer[0..]);
    try bincode.serialize(output_stream.writer(), example);

    // Use an arena to gather allocations from deserializer to make
    // them easy to clean up together. Allocations are required for
    // slices.
    var arena = std.heap.ArenaAllocator.init(std.testing.allocator);
    defer arena.deinit();

    // Read what we wrote
    var input_stream = std.io.fixedBufferStream(output_stream.getWritten());
    const copy = try bincode.deserializeAlloc(
        input_stream.reader(),
        arena.allocator(),
        Shared,
    );

    // Make sure it is the same
    try std.testing.expectEqualStrings("Cat", copy.name);
    try std.testing.expectEqual(@as(u32, 5), copy.age);
}
