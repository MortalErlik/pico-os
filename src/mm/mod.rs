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

pub const SWAP_TOTAL_BYTES: usize = 128 * 1024; // 128 KB Swap Partition
pub const SWAP_TOTAL_PAGES: usize = 32; // 32 * 4KB = 128KB

static mut SWAP_BITMAP: u32 = 0;
static mut SWAP_USED_PAGES: usize = 0;

/// Get swap usage: (used_bytes, total_bytes)
pub fn get_swap_usage() -> (usize, usize) {
    critical_section::with(|_| unsafe {
        (SWAP_USED_PAGES * 4096, SWAP_TOTAL_BYTES)
    })
}

pub fn allocate_swap_page() -> Option<u32> {
    critical_section::with(|_| unsafe {
        for i in 0..SWAP_TOTAL_PAGES {
            let mask = 1u32 << i;
            if SWAP_BITMAP & mask == 0 {
                SWAP_BITMAP |= mask;
                SWAP_USED_PAGES += 1;
                return Some(i as u32);
            }
        }
        None
    })
}

pub fn free_swap_page(page_id: u32) {
    if (page_id as usize) < SWAP_TOTAL_PAGES {
        critical_section::with(|_| unsafe {
            let mask = 1u32 << page_id;
            if SWAP_BITMAP & mask != 0 {
                SWAP_BITMAP &= !mask;
                if SWAP_USED_PAGES > 0 {
                    SWAP_USED_PAGES -= 1;
                }
            }
        });
    }
}
