use std::alloc::GlobalAlloc;

#[cfg(feature = "with-mimalloc")]
#[global_allocator]
pub static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "with-jemalloc")]
#[global_allocator]
pub static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[cfg(not(any(feature = "with-mimalloc", feature = "with-jemalloc")))]
#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::thread::sleep(std::time::Duration::from_secs(1));

    // All the functions exposed by the GlobalAlloc trait (https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html)
    // IMPORTANT: We need the `black_box` to avoid LLVM from optimizing away our allocations

    let emit_marker = || unsafe {
        let layout = std::alloc::Layout::array::<u8>(0xC0D59EED).unwrap();
        let ptr = GLOBAL.alloc(layout);
        core::hint::black_box(ptr);
        GLOBAL.dealloc(ptr, layout);
    };

    emit_marker();

    // alloc (array)
    unsafe {
        let layout = std::alloc::Layout::array::<u8>(4321)?;
        let ptr = GLOBAL.alloc(layout);
        core::hint::black_box(ptr);
        GLOBAL.dealloc(ptr, layout);
    }

    // alloc zeroed (array)
    unsafe {
        let layout = std::alloc::Layout::array::<u8>(1234)?;
        let ptr = GLOBAL.alloc_zeroed(layout);
        core::hint::black_box(ptr);
        GLOBAL.dealloc(ptr, layout);
    }

    // alloc (single value)
    unsafe {
        let layout = std::alloc::Layout::new::<u32>();
        let ptr = GLOBAL.alloc(layout);
        core::hint::black_box(ptr);
        GLOBAL.dealloc(ptr, layout);
    }

    // realloc (allocate new size, copy data, deallocate old)
    unsafe {
        let old_layout = std::alloc::Layout::array::<u8>(1111)?;
        let old_ptr = GLOBAL.alloc(old_layout);

        // Write some data to the old allocation
        std::ptr::write_bytes(old_ptr, 0x42, 1111);
        core::hint::black_box(old_ptr);

        // Reallocate to a larger size
        let new_ptr = GLOBAL.realloc(old_ptr, old_layout, 2222);

        core::hint::black_box(new_ptr);
        GLOBAL.dealloc(new_ptr, std::alloc::Layout::array::<u8>(2222)?);
    }

    emit_marker();

    Ok(())
}
