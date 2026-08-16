//! Virtual File System (VFS) with Dual-Mount Architecture
//!
//! - Root `/`: Fast in-memory rootfs (`/bin`, `/etc`, `/proc`, `/dev`, `/tmp`)
//! - `/data`: Persistent Flash Partition (1.0MB QSPI Flash, persists across reboots)

pub mod flash;

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;
use critical_section::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    NotFound,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    InvalidPath,
    ReadOnly,
    NoSpace,
    Io,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    File { content: Vec<u8> },
    Directory { children: Vec<FsNode> },
}

#[derive(Debug, Clone)]
pub struct FsNode {
    pub name: String,
    pub kind: NodeKind,
}

impl FsNode {
    pub fn new_file(name: &str, content: Vec<u8>) -> Self {
        FsNode {
            name: name.to_string(),
            kind: NodeKind::File { content },
        }
    }

    pub fn new_dir(name: &str) -> Self {
        FsNode {
            name: name.to_string(),
            kind: NodeKind::Directory { children: Vec::new() },
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self.kind, NodeKind::Directory { .. })
    }

    pub fn size(&self) -> usize {
        match &self.kind {
            NodeKind::File { content } => content.len(),
            NodeKind::Directory { children } => children.len() * 32,
        }
    }

    pub fn total_recursive_size(&self) -> usize {
        match &self.kind {
            NodeKind::File { content } => content.len() + self.name.len() + 32,
            NodeKind::Directory { children } => {
                let sum: usize = children.iter().map(|c| c.total_recursive_size()).sum();
                sum + self.name.len() + 64
            }
        }
    }
}

pub struct FileSystem {
    root: FsNode,
    current_dir: String,
}

static GLOBAL_FS: Mutex<RefCell<Option<FileSystem>>> = Mutex::new(RefCell::new(None));

/// Execute an operation with the global FileSystem instance
pub fn with_fs<F, R>(f: F) -> R
where
    F: FnOnce(&mut FileSystem) -> R,
{
    critical_section::with(|cs| {
        let mut fs_lock = GLOBAL_FS.borrow_ref_mut(cs);
        if fs_lock.is_none() {
            let mut fs = FileSystem::new();
            fs.load_from_flash();
            *fs_lock = Some(fs);
        }
        f(fs_lock.as_mut().unwrap())
    })
}

/// Initialize the FileSystem at boot
pub fn init_fs() {
    with_fs(|_fs| {
        // Initialized and loaded from flash
    });
}

/// Format the persistent `/data` Flash partition
pub fn format_fs() -> Result<(), FsError> {
    flash::erase_persist_area();
    with_fs(|fs| {
        let _ = fs.remove("/data", true);
        let _ = fs.create_dir("/data");
        let readme = b"Pico OS Persistent Flash Partition (/data)\n\n\
                       Files stored here are saved on the 2.0MB Physical Flash.\n\
                       They persist across power cycles and reboots!\n\
                       Try: echo 'hello world' > /data/notes.txt\n";
        let _ = fs.write_file("/data/readme.txt", readme);
        fs.save_to_flash();
    });
    Ok(())
}

/// Get filesystem usage: (flash_used_bytes, flash_total_bytes)
pub fn get_fs_usage() -> (usize, usize) {
    let total_flash_bytes = 1024 * 1024; // 1.0 MB Flash Partition for /data
    let used_bytes = with_fs(|fs| {
        if let Ok(node) = fs.find_node("/data") {
            node.total_recursive_size()
        } else {
            0
        }
    });
    (used_bytes, total_flash_bytes)
}

/// Sync all `/data` persistent files to physical Flash
pub fn sync_fs() {
    with_fs(|fs| {
        fs.save_to_flash();
    });
}

impl FileSystem {
    pub fn new() -> Self {
        let mut fs = FileSystem {
            root: FsNode::new_dir("/"),
            current_dir: String::from("/"),
        };
        fs.populate_defaults();
        fs
    }

    fn populate_defaults(&mut self) {
        let _ = self.create_dir("/bin");
        let _ = self.create_dir("/etc");
        let _ = self.create_dir("/home");
        let _ = self.create_dir("/dev");
        let _ = self.create_dir("/proc");
        let _ = self.create_dir("/data");

        let motd = b"====================================================\n\
                     Welcome to Pico OS (Rust + Cortex-M0+ Assembly)\n\
                     Hardware: Raspberry Pi Pico (RP2040 Dual-Core)\n\
                     Root    : tmpfs In-Memory (Ultra-Fast & Stable)\n\
                     Data    : /data Mounted on 1.0MB Persistent Flash\n\
                     RAM     : 264KB SRAM (Free for User Apps)\n\
                     Type 'help' to see available commands.\n\
                     ====================================================\n";
        let _ = self.create_file("/etc/motd", motd.to_vec());

        let os_release = b"NAME=\"Pico OS\"\n\
                           VERSION=\"1.0.0\"\n\
                           ID=picos\n\
                           PRETTY_NAME=\"Pico OS (Dual-Core SMP + FlashFS)\"\n\
                           ARCH=\"armv6-m (Cortex-M0+)\"\n\
                           KERNEL=\"Pico Kernel 1.0\"\n";
        let _ = self.create_file("/etc/os-release", os_release.to_vec());
        let _ = self.create_file("/etc/hostname", b"pico\n".to_vec());

        let readme = b"Pico OS Interactive Shell & File System Guide:\n\n\
                       Available Linux Commands:\n\
                       - ls, cd, pwd, mkdir, rm, touch, cat, cp, mv, echo\n\
                       - ps, kill, htop, free, df, sync, format, uptime, uname, clear\n\
                       - pin, i2c_scan, oled, nano\n\n\
                       Try running 'df -h' to see your /data Flash partition!\n\
                       Try creating files in '/data' to save them permanently!\n";
        let _ = self.create_file("/home/readme.txt", readme.to_vec());

        let data_readme = b"Welcome to your 1.0MB Persistent Flash Storage (/data)!\n\
                            Any file created here will persist across reboots.\n";
        let _ = self.create_file("/data/readme.txt", data_readme.to_vec());
    }

    /// Serialize `/data` partition into Flash
    pub fn save_to_flash(&self) {
        let mut data = Vec::new();
        // Magic header
        data.extend_from_slice(&flash::FLASH_MAGIC.to_le_bytes());

        // Collect all files and directories in `/data`
        let mut entries: Vec<(String, bool, Vec<u8>)> = Vec::new();
        self.collect_entries_recursive("/data", &mut entries);

        let count = entries.len() as u32;
        data.extend_from_slice(&count.to_le_bytes());

        for (path, is_dir, content) in entries {
            let path_bytes = path.as_bytes();
            data.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
            data.extend_from_slice(path_bytes);
            data.push(if is_dir { 1 } else { 0 });
            data.extend_from_slice(&(content.len() as u32).to_le_bytes());
            data.extend_from_slice(&content);
        }

        // Write directly to Flash
        flash::write_flash(0, &data);
    }

    fn collect_entries_recursive(&self, dir_path: &str, entries: &mut Vec<(String, bool, Vec<u8>)>) {
        if let Ok(list) = self.list_dir(dir_path) {
            for entry in list {
                let full_path = if dir_path == "/" {
                    format!("/{}", entry.name)
                } else {
                    format!("{}/{}", dir_path, entry.name)
                };

                if entry.is_dir {
                    entries.push((full_path.clone(), true, Vec::new()));
                    self.collect_entries_recursive(&full_path, entries);
                } else if let Ok(content) = self.read_file(&full_path) {
                    entries.push((full_path, false, content));
                }
            }
        }
    }

    /// Restore `/data` partition from Flash at boot (Zero-allocation direct XIP slice read)
    pub fn load_from_flash(&mut self) {
        let flash_ptr = (0x10000000 + flash::FLASH_PERSIST_OFFSET) as *const u8;
        let header = unsafe { core::slice::from_raw_parts(flash_ptr, 8) };

        let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        if magic != flash::FLASH_MAGIC {
            // Flash not formatted yet, default `/data` will be kept
            return;
        }

        let count = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        if count > 200 {
            // Corrupt header safety guard
            return;
        }

        let payload = unsafe { core::slice::from_raw_parts(flash_ptr.add(8), 65536) };

        let mut offset = 0;
        for _ in 0..count {
            if offset + 2 > payload.len() {
                break;
            }
            let path_len = u16::from_le_bytes([payload[offset], payload[offset + 1]]) as usize;
            offset += 2;

            if offset + path_len + 5 > payload.len() {
                break;
            }
            let path_str = match core::str::from_utf8(&payload[offset..offset + path_len]) {
                Ok(s) => s,
                Err(_) => break,
            };
            offset += path_len;

            let is_dir = payload[offset] == 1;
            offset += 1;

            let content_len = u32::from_le_bytes([
                payload[offset],
                payload[offset + 1],
                payload[offset + 2],
                payload[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + content_len > payload.len() {
                break;
            }
            let content = payload[offset..offset + content_len].to_vec();
            offset += content_len;

            if is_dir {
                let _ = self.create_dir(path_str);
            } else {
                let _ = self.create_file(path_str, content);
            }
        }
    }

    pub fn get_cwd(&self) -> &str {
        &self.current_dir
    }

    pub fn set_cwd(&mut self, path: &str) -> Result<(), FsError> {
        let abs_path = self.normalize_path(path);
        let node = self.find_node(&abs_path)?;
        if node.is_dir() {
            self.current_dir = abs_path;
            Ok(())
        } else {
            Err(FsError::NotADirectory)
        }
    }

    pub fn normalize_path(&self, path: &str) -> String {
        let target = if path.starts_with('/') {
            path.to_string()
        } else if self.current_dir == "/" {
            format!("/{}", path)
        } else {
            format!("{}/{}", self.current_dir, path)
        };

        let mut parts: Vec<&str> = Vec::new();
        for segment in target.split('/') {
            if segment.is_empty() || segment == "." {
                continue;
            }
            if segment == ".." {
                parts.pop();
            } else {
                parts.push(segment);
            }
        }

        if parts.is_empty() {
            String::from("/")
        } else {
            let mut res = String::new();
            for p in parts {
                res.push('/');
                res.push_str(p);
            }
            res
        }
    }

    fn split_path<'a>(&self, path: &'a str) -> (Vec<&'a str>, &'a str) {
        let trimmed = path.trim_matches('/');
        if trimmed.is_empty() {
            return (Vec::new(), "");
        }
        let segments: Vec<&str> = trimmed.split('/').collect();
        let name = segments.last().copied().unwrap_or("");
        let parent_segments = segments[..segments.len().saturating_sub(1)].to_vec();
        (parent_segments, name)
    }

    fn traverse_to_dir_mut(&mut self, segments: &[&str]) -> Result<&mut FsNode, FsError> {
        let mut curr = &mut self.root;
        for &seg in segments {
            if seg.is_empty() {
                continue;
            }
            let next = match &mut curr.kind {
                NodeKind::Directory { children } => {
                    children.iter_mut().find(|c| c.name == seg)
                }
                _ => return Err(FsError::NotADirectory),
            };
            match next {
                Some(n) => {
                    if !n.is_dir() {
                        return Err(FsError::NotADirectory);
                    }
                    curr = n;
                }
                None => return Err(FsError::NotFound),
            }
        }
        Ok(curr)
    }

    pub fn find_node(&self, path: &str) -> Result<&FsNode, FsError> {
        let abs = self.normalize_path(path);
        if abs == "/" {
            return Ok(&self.root);
        }

        let segments: Vec<&str> = abs.trim_matches('/').split('/').collect();
        let mut curr = &self.root;

        for &seg in &segments {
            curr = match &curr.kind {
                NodeKind::Directory { children } => {
                    children.iter().find(|c| c.name == seg).ok_or(FsError::NotFound)?
                }
                _ => return Err(FsError::NotADirectory),
            };
        }
        Ok(curr)
    }

    pub fn create_dir(&mut self, path: &str) -> Result<(), FsError> {
        let abs = self.normalize_path(path);
        if abs == "/" {
            return Err(FsError::AlreadyExists);
        }
        let (parents, name) = self.split_path(&abs);
        let parent = self.traverse_to_dir_mut(&parents)?;

        match &mut parent.kind {
            NodeKind::Directory { children } => {
                if children.iter().any(|c| c.name == name) {
                    return Err(FsError::AlreadyExists);
                }
                children.push(FsNode::new_dir(name));
            }
            _ => return Err(FsError::NotADirectory),
        }
        Ok(())
    }

    pub fn create_file(&mut self, path: &str, content: Vec<u8>) -> Result<(), FsError> {
        let abs = self.normalize_path(path);
        if abs == "/" {
            return Err(FsError::AlreadyExists);
        }
        let (parents, name) = self.split_path(&abs);
        let parent = self.traverse_to_dir_mut(&parents)?;

        match &mut parent.kind {
            NodeKind::Directory { children } => {
                if let Some(existing) = children.iter_mut().find(|c| c.name == name) {
                    if existing.is_dir() {
                        return Err(FsError::IsADirectory);
                    }
                    existing.kind = NodeKind::File { content };
                } else {
                    children.push(FsNode::new_file(name, content));
                }
            }
            _ => return Err(FsError::NotADirectory),
        }
        Ok(())
    }

    pub fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), FsError> {
        self.create_file(path, content.to_vec())
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, FsError> {
        let abs = self.normalize_path(path);

        // Dynamic `/proc` files
        if abs == "/proc/cpuinfo" {
            let info = b"Processor\t: ARMv6-M (Cortex-M0+)\n\
                         BogoMIPS\t: 133.00\n\
                         Features\t: thumb, hardware-spinlock, dual-core, flashfs\n\
                         Hardware\t: RP2040\n\
                         Revision\t: B2\n";
            return Ok(info.to_vec());
        } else if abs == "/proc/meminfo" {
            let stats = crate::mm::get_stats();
            let out = format!(
                "MemTotal:\t{} kB\nMemFree:\t{} kB\nMemUsed:\t{} kB\n",
                stats.total_bytes / 1024,
                stats.free_bytes / 1024,
                stats.used_bytes / 1024
            );
            return Ok(out.into_bytes());
        } else if abs == "/proc/uptime" {
            let ticks = crate::task::get_uptime_ticks();
            let secs = ticks / 1000;
            let out = format!("{}.{:02} seconds\n", secs, (ticks % 1000) / 10);
            return Ok(out.into_bytes());
        } else if abs == "/proc/version" {
            let ver = b"Pico OS version 1.0.0 (markus@pico-os) (rustc baremetal) #1 SMP Dual-Mount\n";
            return Ok(ver.to_vec());
        }

        let node = self.find_node(&abs)?;
        match &node.kind {
            NodeKind::File { content } => Ok(content.clone()),
            NodeKind::Directory { .. } => Err(FsError::IsADirectory),
        }
    }

    pub fn list_dir(&self, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let abs = self.normalize_path(path);

        if abs == "/proc" {
            return Ok(alloc::vec![
                DirEntry { name: String::from("cpuinfo"), is_dir: false, size: 140 },
                DirEntry { name: String::from("meminfo"), is_dir: false, size: 64 },
                DirEntry { name: String::from("uptime"), is_dir: false, size: 20 },
                DirEntry { name: String::from("version"), is_dir: false, size: 78 },
            ]);
        } else if abs == "/dev" {
            return Ok(alloc::vec![
                DirEntry { name: String::from("null"), is_dir: false, size: 0 },
                DirEntry { name: String::from("zero"), is_dir: false, size: 0 },
                DirEntry { name: String::from("random"), is_dir: false, size: 0 },
                DirEntry { name: String::from("flash"), is_dir: false, size: 1048576 },
                DirEntry { name: String::from("ttyACM0"), is_dir: false, size: 0 },
            ]);
        }

        let node = self.find_node(&abs)?;
        match &node.kind {
            NodeKind::Directory { children } => {
                let mut entries: Vec<DirEntry> = children
                    .iter()
                    .map(|c| DirEntry {
                        name: c.name.clone(),
                        is_dir: c.is_dir(),
                        size: c.size(),
                    })
                    .collect();
                entries.sort_by(|a, b| {
                    if a.is_dir == b.is_dir {
                        a.name.cmp(&b.name)
                    } else if a.is_dir {
                        core::cmp::Ordering::Less
                    } else {
                        core::cmp::Ordering::Greater
                    }
                });
                Ok(entries)
            }
            NodeKind::File { .. } => Err(FsError::NotADirectory),
        }
    }

    pub fn remove(&mut self, path: &str, recursive: bool) -> Result<(), FsError> {
        let abs = self.normalize_path(path);
        if abs == "/" {
            return Err(FsError::ReadOnly);
        }
        let (parents, name) = self.split_path(&abs);
        let parent = self.traverse_to_dir_mut(&parents)?;

        match &mut parent.kind {
            NodeKind::Directory { children } => {
                let idx = children.iter().position(|c| c.name == name).ok_or(FsError::NotFound)?;
                let target = &children[idx];
                if target.is_dir() {
                    if let NodeKind::Directory { children: ref sub } = target.kind {
                        if !sub.is_empty() && !recursive {
                            return Err(FsError::IsADirectory);
                        }
                    }
                }
                children.remove(idx);
            }
            _ => return Err(FsError::NotADirectory),
        }
        Ok(())
    }

    pub fn copy(&mut self, src: &str, dst: &str) -> Result<(), FsError> {
        let data = self.read_file(src)?;
        self.write_file(dst, &data)
    }

    pub fn move_node(&mut self, src: &str, dst: &str) -> Result<(), FsError> {
        let data = self.read_file(src)?;
        self.write_file(dst, &data)?;
        self.remove(src, false)
    }
}
