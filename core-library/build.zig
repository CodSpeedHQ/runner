const std = @import("std");

pub fn build(b: *std.Build) void {
    const optimize = b.standardOptimizeOption(.{});
    const target = b.standardTargetOptions(.{ .default_target = .{ .ofmt = .c } });

    // Core Library
    //
    const libcore = b.addStaticLibrary(.{
        .name = "core",
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    libcore.addIncludePath(b.path("src/includes"));
    libcore.addCSourceFile(.{
        .file = b.path("src/includes/wrapper.c"),
        .flags = &.{"-Wall"},
    });
    b.installArtifact(libcore);
    b.installFile("src/includes/wrapper.c", "lib/wrapper.c");
    b.installFile("src/includes/valgrind.h", "lib/valgrind.h");
    b.installFile("src/includes/callgrind.h", "lib/callgrind.h");
    b.installFile("src/includes/core.h", "lib/core.h");
    // TODO: Copy zig.h

    // Tests
    //
    const test_main = b.addTest(.{
        .root_source_file = b.path("src/root.zig"),
        .optimize = optimize,
        .link_libc = true,
        .test_runner = b.path("src/test_main.zig")
    });
    test_main.linkLibC();
    test_main.addIncludePath(b.path("src/includes"));
    test_main.addCSourceFile(.{
        .file = b.path("src/includes/wrapper.c"),
        .flags = &.{"-Wall"},
    });
    const run_test_main = b.addRunArtifact(test_main);
    b.step("test", "test utility functions").dependOn(&run_test_main.step);
}
