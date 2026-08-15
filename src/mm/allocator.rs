//! Custom Heap Allocator for Pico OS
//! Implements a first-fit linked list allocator with block coalescing
//! and memory usage statistics.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;

/// Alignment requirement for Cortex-M (8 bytes)
const ALIGN: usize = 8;

/// Minimum block size including header
const HEADER_SIZE: usize = core::mem::size_of::<BlockHeader>();

#[repr(C)]
struct BlockHeader {
    size: usize,        // Payload size in bytes
    is_free: bool,      // Free flag
    next: *mut BlockHeader,
}

pub struct MemoryStats {
    pub total_bytes: usize,
    pub used_bytes: usize,
    pub free_bytes: usize,
    pub peak_used_bytes: usize,
    pub alloc_count: usize,
    pub free_count: usize,
}

pub struct CustomHeap {
    head: *mut BlockHeader,
    heap_start: usize,
    heap_end: usize,
    total_bytes: usize,
    used_bytes: usize,
    peak_used_bytes: usize,
    alloc_count: usize,
    free_count: usize,
}

impl CustomHeap {
    pub const fn empty() -> Self {
        CustomHeap {
            head: core::ptr::null_mut(),
            heap_start: 0,
            heap_end: 0,
            total_bytes: 0,
            used_bytes: 0,
            peak_used_bytes: 0,
            alloc_count: 0,
            free_count: 0,
        }
    }

    /// Initialize the heap with a given memory region
    ///
    /// # Safety
    /// Caller must ensure the memory region is valid and exclusively owned by the heap.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        let aligned_start = (heap_start + (ALIGN - 1)) & !(ALIGN - 1);
        let end = heap_start + heap_size;
        let aligned_size = if end > aligned_start + HEADER_SIZE {
            end - aligned_start
        } else {
            0
        };

        self.heap_start = aligned_start;
        self.heap_end = end;
        self.total_bytes = aligned_size;
        self.used_bytes = 0;
        self.peak_used_bytes = 0;
        self.alloc_count = 0;
        self.free_count = 0;

        if aligned_size > HEADER_SIZE {
            let initial_block = aligned_start as *mut BlockHeader;
            (*initial_block).size = aligned_size - HEADER_SIZE;
            (*initial_block).is_free = true;
            (*initial_block).next = core::ptr::null_mut();
            self.head = initial_block;
        } else {
            self.head = core::ptr::null_mut();
        }
    }

    /// Allocate memory of `layout.size()` with required alignment
    pub fn allocate(&mut self, layout: Layout) -> *mut u8 {
        let _align = layout.align().max(ALIGN);
        let size = (layout.size() + (ALIGN - 1)) & !(ALIGN - 1);

        let mut curr = self.head;

        while !curr.is_null() {
            unsafe {
                let header = &mut *curr;
                if header.is_free && header.size >= size {
                    let remaining = header.size - size;
                    if remaining >= HEADER_SIZE + ALIGN {
                        let next_addr = (curr as usize) + HEADER_SIZE + size;
                        let new_block = next_addr as *mut BlockHeader;
                        (*new_block).size = remaining - HEADER_SIZE;
                        (*new_block).is_free = true;
                        (*new_block).next = header.next;

                        header.size = size;
                        header.is_free = false;
                        header.next = new_block;
                    } else {
                        header.is_free = false;
                    }

                    self.used_bytes += header.size + HEADER_SIZE;
                    if self.used_bytes > self.peak_used_bytes {
                        self.peak_used_bytes = self.used_bytes;
                    }
                    self.alloc_count += 1;

                    let payload_ptr = (curr as usize + HEADER_SIZE) as *mut u8;
                    return payload_ptr;
                }
                curr = header.next;
            }
        }

        core::ptr::null_mut()
    }

    /// Deallocate memory block
    pub fn deallocate(&mut self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }

        unsafe {
            let block_addr = (ptr as usize) - HEADER_SIZE;
            if block_addr < self.heap_start || block_addr >= self.heap_end {
                return;
            }

            let block = block_addr as *mut BlockHeader;
            (*block).is_free = true;
            self.free_count += 1;
            if self.used_bytes >= (*block).size + HEADER_SIZE {
                self.used_bytes -= (*block).size + HEADER_SIZE;
            }

            self.coalesce();
        }
    }

    unsafe fn coalesce(&mut self) {
        let mut curr = self.head;

        while !curr.is_null() {
            let header = &mut *curr;
            if header.is_free {
                let next = header.next;
                if !next.is_null() && (*next).is_free {
                    let next_addr = next as usize;
                    let expected_addr = (curr as usize) + HEADER_SIZE + header.size;
                    if next_addr == expected_addr {
                        header.size += HEADER_SIZE + (*next).size;
                        header.next = (*next).next;
                        continue;
                    }
                }
            }
            curr = header.next;
        }
    }

    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            total_bytes: self.total_bytes,
            used_bytes: self.used_bytes,
            free_bytes: if self.total_bytes >= self.used_bytes {
                self.total_bytes - self.used_bytes
            } else {
                0
            },
            peak_used_bytes: self.peak_used_bytes,
            alloc_count: self.alloc_count,
            free_count: self.free_count,
        }
    }
}

pub struct LockedHeap {
    inner: UnsafeCell<CustomHeap>,
}

unsafe impl Sync for LockedHeap {}
unsafe impl Send for LockedHeap {}

impl LockedHeap {
    pub const fn empty() -> Self {
        LockedHeap {
            inner: UnsafeCell::new(CustomHeap::empty()),
        }
    }

    pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        critical_section::with(|_| {
            let heap = &mut *self.inner.get();
            heap.init(heap_start, heap_size);
        });
    }

    pub fn stats(&self) -> MemoryStats {
        critical_section::with(|_| {
            let heap = unsafe { &*self.inner.get() };
            heap.stats()
        })
    }
}

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        critical_section::with(|_| {
            let heap = &mut *self.inner.get();
            heap.allocate(layout)
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        critical_section::with(|_| {
            let heap = &mut *self.inner.get();
            heap.deallocate(ptr, layout);
        });
    }
}
