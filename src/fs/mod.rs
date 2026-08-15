//! Virtual File System (VFS) for Pico OS
//! Implements a hierarchical in-memory filesystem with directory navigation,
//! file creation, deletion, reading, writing, and pre-populated Unix structure.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    NotFound,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    InvalidPath,
    ReadOnly,
    NoSpace,
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
            NodeKind::Directory { children } => children.len() * 32, // nominal directory size
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

        let motd = b"====================================================\n\
                     Welcome to Pico OS (Rust + Cortex-M0+ Assembly)\n\
                     Hardware: Raspberry Pi Pico (RP2040 Dual-Core)\n\
                     Type 'help' to see available commands.\n\
                     ====================================================\n";
        let _ = self.create_file("/etc/motd", motd.to_vec());

        let os_release = b"NAME=\"Pico OS\"\n\
                           VERSION=\"1.0.0\"\n\
                           ID=picos\n\
                           PRETTY_NAME=\"Pico OS (Rust/ASM on RP2040)\"\n\
                           ARCH=\"armv6-m (Cortex-M0+)\"\n\
                           KERNEL=\"Pico Kernel 1.0\"\n";
        let _ = self.create_file("/etc/os-release", os_release.to_vec());
        let _ = self.create_file("/etc/hostname", b"pico\n".to_vec());

        let readme = b"Pico OS Interactive Shell & File System Guide:\n\n\
                       Available Linux Commands:\n\
                       - ls, cd, pwd, mkdir, rm, touch, cat, cp, mv, echo\n\
                       - ps, kill, htop, free, uptime, uname, whoami, clear, reboot\n\
                       - pin, i2c_scan, oled, nano\n\n\
                       Try running 'htop' to see real-time task & RAM stats!\n\
                       Try running 'nano file.txt' to create and edit files!\n";
        let _ = self.create_file("/home/readme.txt", readme.to_vec());

        let proc_cpuinfo = b"Processor\t: ARMv6-M (Cortex-M0+)\n\
                             BogoMIPS\t: 133.00\n\
                             Features\t: thumb, hardware-spinlock, dual-core\n\
                             Hardware\t: RP2040\n\
                             Revision\t: B2\n";
        let _ = self.create_file("/proc/cpuinfo", proc_cpuinfo.to_vec());
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
                Ok(())
            }
            _ => Err(FsError::NotADirectory),
        }
    }

    pub fn create_file(&mut self, path: &str, content: Vec<u8>) -> Result<(), FsError> {
        let abs = self.normalize_path(path);
        let (parents, name) = self.split_path(&abs);
        let parent = self.traverse_to_dir_mut(&parents)?;

        match &mut parent.kind {
            NodeKind::Directory { children } => {
                if let Some(existing) = children.iter_mut().find(|c| c.name == name) {
                    if existing.is_dir() {
                        return Err(FsError::IsADirectory);
                    }
                    existing.kind = NodeKind::File { content };
                    Ok(())
                } else {
                    children.push(FsNode::new_file(name, content));
                    Ok(())
                }
            }
            _ => Err(FsError::NotADirectory),
        }
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, FsError> {
        let node = self.find_node(path)?;
        match &node.kind {
            NodeKind::File { content } => Ok(content.clone()),
            NodeKind::Directory { .. } => Err(FsError::IsADirectory),
        }
    }

    pub fn write_file(&mut self, path: &str, content: Vec<u8>) -> Result<(), FsError> {
        self.create_file(path, content)
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
                if children[idx].is_dir() && !recursive {
                    if let NodeKind::Directory { children: sub } = &children[idx].kind {
                        if !sub.is_empty() {
                            return Err(FsError::AlreadyExists); // Directory not empty
                        }
                    }
                }
                children.remove(idx);
                Ok(())
            }
            _ => Err(FsError::NotADirectory),
        }
    }

    pub fn list_dir(&self, path: &str) -> Result<Vec<DirEntry>, FsError> {
        let node = self.find_node(path)?;
        match &node.kind {
            NodeKind::Directory { children } => {
                let mut entries = Vec::new();
                for c in children {
                    entries.push(DirEntry {
                        name: c.name.clone(),
                        is_dir: c.is_dir(),
                        size: c.size(),
                    });
                }
                Ok(entries)
            }
            NodeKind::File { .. } => Err(FsError::NotADirectory),
        }
    }

    pub fn copy(&mut self, src_path: &str, dst_path: &str) -> Result<(), FsError> {
        let content = self.read_file(src_path)?;
        self.create_file(dst_path, content)
    }

    pub fn move_node(&mut self, src_path: &str, dst_path: &str) -> Result<(), FsError> {
        self.copy(src_path, dst_path)?;
        self.remove(src_path, true)
    }

    pub fn get_total_size(&self) -> usize {
        self.root.total_recursive_size()
    }
}

static mut FS: Option<FileSystem> = None;

pub fn init_fs() {
    unsafe {
        FS = Some(FileSystem::new());
    }
}

pub fn get_fs_usage() -> usize {
    critical_section::with(|_| unsafe {
        if let Some(ref fs) = FS {
            fs.get_total_size()
        } else {
            0
        }
    })
}

pub fn with_fs<F, R>(f: F) -> R
where
    F: FnOnce(&mut FileSystem) -> R,
{
    critical_section::with(|_| unsafe {
        if let Some(ref mut fs) = FS {
            f(fs)
        } else {
            panic!("FileSystem not initialized");
        }
    })
}
