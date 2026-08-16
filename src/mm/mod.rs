pub mod allocator;

pub use allocator::{LockedHeap, MemoryStats};

#[global_allocator]
pub static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initializes the global heap in SRAM.
/// On RP2040, we allocate 216 KB of SRAM for the operating system heap.
pub fn init_heap() {
    const HEAP_SIZE: usize = 192 * 1024; // 192 KB (leaves 64KB for stacks & DMA)
    static mut HEAP_MEM: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

    unsafe {
        HEAP_ALLOCATOR.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
    }
}

pub fn get_stats() -> MemoryStats {
    HEAP_ALLOCATOR.stats()
}
