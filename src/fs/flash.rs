//! RP2040 Hardware Flash Driver for Persistent Storage
//!
//! Executes sector erase and page program routines 100% from SRAM (.data)
//! with Boot ROM function pointers, inline Cortex-M0+ assembly, and hardware XIP cache restoration.

use core::sync::atomic::{AtomicBool, Ordering};

// Flash Memory Map Offsets
pub const FLASH_VFS_OFFSET: u32 = 0x40000;    // 256 KB Offset: For VFS Snapshot (256 KB size)
pub const FLASH_SWAP_OFFSET: u32 = 0x80000;   // 512 KB Offset: For Swap Partition (128 KB size)
pub const FLASH_DISK_OFFSET: u32 = 0xA0000;   // 640 KB Offset: For True Disk Partition (1.4 MB size)

pub const FLASH_SECTOR_SIZE: usize = 4096;
pub const FLASH_MAGIC: u32 = 0x5049434F; // "PICO"

/// Synchronization flags between Core 0 and Core 1
pub static FLASH_LOCKOUT: AtomicBool = AtomicBool::new(false);
pub static CORE1_ACK: AtomicBool = AtomicBool::new(false);
pub static CORE1_SPAWNED: AtomicBool = AtomicBool::new(false);

/// Function placed in SRAM so Core 1 spins safely while Core 0 disables XIP
#[inline(never)]
#[link_section = ".data"]
pub fn core1_check_flash_lockout() {
    if FLASH_LOCKOUT.load(Ordering::Relaxed) {
        CORE1_ACK.store(true, Ordering::SeqCst);
        while FLASH_LOCKOUT.load(Ordering::Relaxed) {
            cortex_m::asm::nop();
        }
        CORE1_ACK.store(false, Ordering::SeqCst);
    }
}

pub fn begin_transaction() {
    if !CORE1_SPAWNED.load(Ordering::Relaxed) {
        return;
    }
    while CORE1_ACK.load(Ordering::Relaxed) {
        cortex_m::asm::nop();
    }
    FLASH_LOCKOUT.store(true, Ordering::SeqCst);
    while !CORE1_ACK.load(Ordering::Relaxed) {
        cortex_m::asm::nop();
    }
}

pub fn end_transaction() {
    if !CORE1_SPAWNED.load(Ordering::Relaxed) {
        return;
    }
    FLASH_LOCKOUT.store(false, Ordering::SeqCst);
    while CORE1_ACK.load(Ordering::Relaxed) {
        cortex_m::asm::nop();
    }
    cortex_m::asm::delay(1000);
}

#[repr(C, align(4))]
struct AlignedPageBuf([u8; 256]);
static mut PAGE_BUF: AlignedPageBuf = AlignedPageBuf([0; 256]);

#[repr(C)]
pub struct RomFns {
    pub connect: unsafe extern "C" fn(),
    pub exit_xip: unsafe extern "C" fn(),
    pub erase: unsafe extern "C" fn(u32, usize, u32, u8),
    pub prog: unsafe extern "C" fn(u32, *const u8, usize),
    pub flush: unsafe extern "C" fn(),
    pub enter_xip: unsafe extern "C" fn(),
}

impl RomFns {
    pub fn load() -> Self {
        Self {
            connect: rp2040_hal::rom_data::connect_internal_flash::ptr(),
            exit_xip: rp2040_hal::rom_data::flash_exit_xip::ptr(),
            erase: rp2040_hal::rom_data::flash_range_erase::ptr(),
            prog: rp2040_hal::rom_data::flash_range_program::ptr(),
            flush: rp2040_hal::rom_data::flash_flush_cache::ptr(),
            enter_xip: rp2040_hal::rom_data::flash_enter_cmd_xip::ptr(),
        }
    }
}

/// RAM-resident Flash Erase routine
#[inline(never)]
#[link_section = ".data"]
unsafe fn ram_erase_sector(flash_offset: u32, fns: &RomFns) {
    (fns.connect)();
    (fns.exit_xip)();
    (fns.erase)(flash_offset, 4096, 4096, 0x20);
    (fns.flush)();
    (fns.enter_xip)();
}

/// RAM-resident Flash Program routine
#[inline(never)]
#[link_section = ".data"]
unsafe fn ram_program_page(flash_offset: u32, data_ptr: *const u8, fns: &RomFns) {
    (fns.connect)();
    (fns.exit_xip)();
    (fns.prog)(flash_offset, data_ptr, 256);
    (fns.flush)();
    (fns.enter_xip)();
}

/// Read directly from XIP memory mapped Flash (0x10000000 + offset)
pub fn read_flash(offset: u32, buf: &mut [u8]) {
    let xip_ptr = (0x10000000 + FLASH_VFS_OFFSET + offset) as *const u8;
    unsafe {
        core::ptr::copy_nonoverlapping(xip_ptr, buf.as_mut_ptr(), buf.len());
    }
}

/// Erase sectors and write raw bytes into Flash in a single atomic transaction
pub fn write_flash(offset: u32, data: &[u8]) -> bool {
    let fns = RomFns::load();

    let total_sectors = (data.len() + FLASH_SECTOR_SIZE - 1) / FLASH_SECTOR_SIZE;
    let total_pages = (data.len() + 255) / 256;
    let base_flash_offset = FLASH_VFS_OFFSET + offset;

    // Single atomic lockout transaction for the entire flash write operation
    begin_transaction();
    cortex_m::interrupt::free(|_| unsafe {
        // 1. Erase all required sectors
        for i in 0..total_sectors {
            let sector_offset = base_flash_offset + (i * FLASH_SECTOR_SIZE) as u32;
            ram_erase_sector(sector_offset, &fns);
        }

        // 2. Program all 256-byte pages
        for p in 0..total_pages {
            let page_offset = base_flash_offset + (p * 256) as u32;
            let start = p * 256;
            let end = (start + 256).min(data.len());
            let chunk = &data[start..end];

            let page_buf_ptr = core::ptr::addr_of_mut!(PAGE_BUF.0);
            (*page_buf_ptr).fill(0xFF);
            core::ptr::copy_nonoverlapping(chunk.as_ptr(), (*page_buf_ptr).as_mut_ptr(), chunk.len());
            ram_program_page(page_offset, (*page_buf_ptr).as_ptr(), &fns);
        }
    });
    end_transaction();

    true
}

/// Erase the persistent flash storage area in a single atomic transaction
pub fn erase_persist_area() {
    let fns = RomFns::load();

    begin_transaction();
    cortex_m::interrupt::free(|_| unsafe {
        // Erase first 8 sectors (32 KB) which holds the snapshot
        for i in 0..8 {
            let sector_offset = FLASH_VFS_OFFSET + (i * FLASH_SECTOR_SIZE) as u32;
            ram_erase_sector(sector_offset, &fns);
        }
    });
    end_transaction();
}

/// Read a 4096-byte block from the True Disk Partition
pub fn read_disk_block(block_id: u32, buf: &mut [u8; 4096]) {
    let offset = block_id * (FLASH_SECTOR_SIZE as u32);
    let xip_ptr = (0x10000000 + FLASH_DISK_OFFSET + offset) as *const u8;
    unsafe {
        core::ptr::copy_nonoverlapping(xip_ptr, buf.as_mut_ptr(), buf.len());
    }
}

/// Erase and write a 4096-byte block to the True Disk Partition
pub fn write_disk_block(block_id: u32, buf: &[u8; 4096]) {
    let fns = RomFns::load();
    let offset = block_id * (FLASH_SECTOR_SIZE as u32);
    let base_flash_offset = FLASH_DISK_OFFSET + offset;

    begin_transaction();
    cortex_m::interrupt::free(|_| unsafe {
        // Erase 1 sector (4096 bytes)
        ram_erase_sector(base_flash_offset, &fns);

        // Program 16 pages (256 bytes * 16 = 4096 bytes)
        for p in 0..16 {
            let page_offset = base_flash_offset + (p * 256) as u32;
            let chunk = &buf[(p as usize * 256)..((p + 1) as usize * 256)];
            let page_buf_ptr = core::ptr::addr_of_mut!(PAGE_BUF.0);
            core::ptr::copy_nonoverlapping(chunk.as_ptr(), (*page_buf_ptr).as_mut_ptr(), chunk.len());
            ram_program_page(page_offset, (*page_buf_ptr).as_ptr(), &fns);
        }
    });
    end_transaction();
}

/// Read a 4096-byte block from the Swap Partition
pub fn read_swap_block(block_id: u32, buf: &mut [u8; 4096]) {
    let offset = block_id * (FLASH_SECTOR_SIZE as u32);
    let xip_ptr = (0x10000000 + FLASH_SWAP_OFFSET + offset) as *const u8;
    unsafe {
        core::ptr::copy_nonoverlapping(xip_ptr, buf.as_mut_ptr(), buf.len());
    }
}

/// Erase and write a 4096-byte block to the Swap Partition
pub fn write_swap_block(block_id: u32, buf: &[u8; 4096]) {
    let fns = RomFns::load();
    let offset = block_id * (FLASH_SECTOR_SIZE as u32);
    let base_flash_offset = FLASH_SWAP_OFFSET + offset;

    begin_transaction();
    cortex_m::interrupt::free(|_| unsafe {
        ram_erase_sector(base_flash_offset, &fns);
        for p in 0..16 {
            let page_offset = base_flash_offset + (p * 256) as u32;
            let chunk = &buf[(p as usize * 256)..((p + 1) as usize * 256)];
            let page_buf_ptr = core::ptr::addr_of_mut!(PAGE_BUF.0);
            core::ptr::copy_nonoverlapping(chunk.as_ptr(), (*page_buf_ptr).as_mut_ptr(), chunk.len());
            ram_program_page(page_offset, (*page_buf_ptr).as_ptr(), &fns);
        }
    });
    end_transaction();
}
