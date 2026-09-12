//! Thin VFS + in-RAM memfs (Track A / A6 / ADR-027).
//!
//! One backend this mile: named heap buffers. Not Linux VFS. Not POSIX.
//! Not virtio-blk. Not FAT. Not app hosting.

use alloc::vec::Vec;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use spin::Mutex;

use crate::el0;
use crate::paging;
use crate::syscall;
use crate::uart;

/// Flat path: `/` + 1..=15 of `[a-z0-9_-]`.
pub const PATH_MAX: usize = 16;
/// Per-file cap. Named buffers, not a disk.
pub const FILE_MAX: usize = 256;
const MAX_FILES: usize = 8;
const MAX_HANDLES: usize = 4;

const KERNEL_PATH: &str = "/kprobe";
const KERNEL_BYTES: &[u8] = b"memfs-hi";
#[cfg_attr(not(test), allow(dead_code))]
const EL0_PATH: &str = "/eprobe";
const EL0_BYTES: &[u8] = b"memfs-el0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsError {
    BadPath,
    Exists,
    Missing,
    Full,
    BadHandle,
    TooBig,
}

/// Thin VFS. A later block FS may implement the same five calls.
pub trait VfsOps {
    fn create(&mut self, path: &str) -> Result<u32, FsError>;
    fn open(&mut self, path: &str) -> Result<u32, FsError>;
    fn read(&mut self, fd: u32, buf: &mut [u8]) -> Result<usize, FsError>;
    fn write(&mut self, fd: u32, buf: &[u8]) -> Result<usize, FsError>;
    fn close(&mut self, fd: u32) -> Result<(), FsError>;
}

struct File {
    name: [u8; PATH_MAX],
    name_len: u8,
    data: Vec<u8>,
}

impl File {
    fn new(path: &str) -> Option<Self> {
        if !valid_path(path) {
            return None;
        }
        let mut name = [0u8; PATH_MAX];
        name[..path.len()].copy_from_slice(path.as_bytes());
        Some(Self {
            name,
            name_len: path.len() as u8,
            data: Vec::new(),
        })
    }

    fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("")
    }
}

#[derive(Clone, Copy)]
struct Handle {
    used: bool,
    file: usize,
    off: usize,
}

impl Handle {
    const fn empty() -> Self {
        Self {
            used: false,
            file: 0,
            off: 0,
        }
    }
}

struct MemFs {
    files: [Option<File>; MAX_FILES],
    handles: [Handle; MAX_HANDLES],
}

impl MemFs {
    const fn empty() -> Self {
        Self {
            files: [None, None, None, None, None, None, None, None],
            handles: [
                Handle::empty(),
                Handle::empty(),
                Handle::empty(),
                Handle::empty(),
            ],
        }
    }

    fn reset(&mut self) {
        self.files = [None, None, None, None, None, None, None, None];
        self.handles = [
            Handle::empty(),
            Handle::empty(),
            Handle::empty(),
            Handle::empty(),
        ];
    }

    fn find_name(&self, path: &str) -> Option<usize> {
        self.files.iter().position(|f| {
            f.as_ref()
                .map(|file| file.name() == path)
                .unwrap_or(false)
        })
    }

    fn alloc_handle(&mut self, file: usize) -> Result<u32, FsError> {
        for (i, h) in self.handles.iter_mut().enumerate() {
            if !h.used {
                *h = Handle {
                    used: true,
                    file,
                    off: 0,
                };
                return Ok((i as u32) + 1);
            }
        }
        Err(FsError::Full)
    }

    fn handle_mut(&mut self, fd: u32) -> Result<&mut Handle, FsError> {
        if fd == 0 || fd as usize > MAX_HANDLES {
            return Err(FsError::BadHandle);
        }
        let h = &mut self.handles[(fd as usize) - 1];
        if !h.used {
            return Err(FsError::BadHandle);
        }
        Ok(h)
    }
}

impl VfsOps for MemFs {
    fn create(&mut self, path: &str) -> Result<u32, FsError> {
        if !valid_path(path) {
            return Err(FsError::BadPath);
        }
        if self.find_name(path).is_some() {
            return Err(FsError::Exists);
        }
        let slot = self
            .files
            .iter()
            .position(|f| f.is_none())
            .ok_or(FsError::Full)?;
        let file = File::new(path).ok_or(FsError::BadPath)?;
        self.files[slot] = Some(file);
        self.alloc_handle(slot)
    }

    fn open(&mut self, path: &str) -> Result<u32, FsError> {
        if !valid_path(path) {
            return Err(FsError::BadPath);
        }
        let slot = self.find_name(path).ok_or(FsError::Missing)?;
        self.alloc_handle(slot)
    }

    fn read(&mut self, fd: u32, buf: &mut [u8]) -> Result<usize, FsError> {
        let (slot, off) = {
            let h = self.handle_mut(fd)?;
            (h.file, h.off)
        };
        let file = self.files[slot].as_ref().ok_or(FsError::BadHandle)?;
        if off > file.data.len() {
            return Err(FsError::BadHandle);
        }
        let n = core::cmp::min(buf.len(), file.data.len() - off);
        buf[..n].copy_from_slice(&file.data[off..off + n]);
        self.handle_mut(fd)?.off = off + n;
        Ok(n)
    }

    fn write(&mut self, fd: u32, buf: &[u8]) -> Result<usize, FsError> {
        if buf.is_empty() {
            return Err(FsError::TooBig);
        }
        let (slot, off) = {
            let h = self.handle_mut(fd)?;
            (h.file, h.off)
        };
        let file = self.files[slot].as_mut().ok_or(FsError::BadHandle)?;
        if off > file.data.len() {
            return Err(FsError::BadHandle);
        }
        let room = FILE_MAX.saturating_sub(off);
        if room == 0 {
            return Err(FsError::TooBig);
        }
        let n = core::cmp::min(buf.len(), room);
        if off + n > file.data.len() {
            file.data.resize(off + n, 0);
        }
        file.data[off..off + n].copy_from_slice(&buf[..n]);
        self.handle_mut(fd)?.off = off + n;
        Ok(n)
    }

    fn close(&mut self, fd: u32) -> Result<(), FsError> {
        let h = self.handle_mut(fd)?;
        *h = Handle::empty();
        Ok(())
    }
}

static FS: Mutex<MemFs> = Mutex::new(MemFs::empty());

static CREATE_OK: AtomicBool = AtomicBool::new(false);
static WRITE_OK: AtomicBool = AtomicBool::new(false);
static READ_OK: AtomicBool = AtomicBool::new(false);
static EL0_OK: AtomicBool = AtomicBool::new(false);
static LAST_READ: Mutex<[u8; 16]> = Mutex::new([0; 16]);
static LAST_READ_LEN: AtomicU8 = AtomicU8::new(0);
static LAST_EL0_READ: AtomicU64 = AtomicU64::new(u64::MAX);

pub fn init() {
    reset();
}

pub fn reset() {
    FS.lock().reset();
}

pub fn valid_path(path: &str) -> bool {
    let b = path.as_bytes();
    if b.len() < 2 || b.len() > PATH_MAX || b[0] != b'/' {
        return false;
    }
    b[1..].iter().all(|&c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-'
    })
}

pub fn create(path: &str) -> Result<u32, FsError> {
    FS.lock().create(path)
}

pub fn open(path: &str) -> Result<u32, FsError> {
    FS.lock().open(path)
}

pub fn read(fd: u32, buf: &mut [u8]) -> Result<usize, FsError> {
    FS.lock().read(fd, buf)
}

pub fn write(fd: u32, buf: &[u8]) -> Result<usize, FsError> {
    FS.lock().write(fd, buf)
}

pub fn close(fd: u32) -> Result<(), FsError> {
    FS.lock().close(fd)
}

fn kernel_roundtrip() -> bool {
    reset();
    CREATE_OK.store(false, Ordering::SeqCst);
    WRITE_OK.store(false, Ordering::SeqCst);
    READ_OK.store(false, Ordering::SeqCst);
    let Ok(fd) = create(KERNEL_PATH) else {
        return false;
    };
    if fd == 0 {
        return false;
    }
    uart::write_str_raw("fs: create\n");
    CREATE_OK.store(true, Ordering::SeqCst);
    let Ok(n) = write(fd, KERNEL_BYTES) else {
        let _ = close(fd);
        return false;
    };
    if n != KERNEL_BYTES.len() {
        let _ = close(fd);
        return false;
    }
    uart::write_str_raw("fs: write\n");
    WRITE_OK.store(true, Ordering::SeqCst);
    if close(fd).is_err() {
        return false;
    }
    let Ok(fd) = open(KERNEL_PATH) else {
        return false;
    };
    let mut buf = [0u8; 16];
    let Ok(n) = read(fd, &mut buf) else {
        let _ = close(fd);
        return false;
    };
    let _ = close(fd);
    if n != KERNEL_BYTES.len() || &buf[..n] != KERNEL_BYTES {
        return false;
    }
    uart::write_str_raw("fs: read\n");
    READ_OK.store(true, Ordering::SeqCst);
    true
}

fn record_el0_read(buf: &[u8]) {
    let n = core::cmp::min(buf.len(), 16);
    {
        let mut slot = LAST_READ.lock();
        slot[..n].copy_from_slice(&buf[..n]);
        slot[n..].fill(0);
    }
    LAST_READ_LEN.store(n as u8, Ordering::SeqCst);
    LAST_EL0_READ.store(n as u64, Ordering::SeqCst);
}

/// Called from the EL0 `fs_read` path after a successful copy-out.
pub(crate) fn note_el0_read(buf: &[u8]) {
    record_el0_read(buf);
}

fn el0_roundtrip() -> bool {
    if el0::is_active() || !paging::user_map_ready() {
        return false;
    }
    EL0_OK.store(false, Ordering::SeqCst);
    LAST_EL0_READ.store(u64::MAX, Ordering::SeqCst);
    LAST_READ_LEN.store(0, Ordering::SeqCst);
    syscall::reset_fs_flags();
    if !syscall::run_fs_el0_payload() {
        return false;
    }
    let n = LAST_READ_LEN.load(Ordering::SeqCst) as usize;
    let got = *LAST_READ.lock();
    if n != EL0_BYTES.len() || &got[..n] != EL0_BYTES {
        return false;
    }
    if syscall::last_fs_create() == 0
        || syscall::last_fs_write() != EL0_BYTES.len() as u64
        || syscall::last_fs_open() == 0
        || syscall::last_fs_read() != EL0_BYTES.len() as u64
        || !syscall::fs_close_seen()
    {
        return false;
    }
    uart::write_str_raw("fs: el0\n");
    EL0_OK.store(true, Ordering::SeqCst);
    true
}

/// Serial proof: kernel + EL0 create/write/read/close. Not FAT. Not app hosting.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() {
        return false;
    }
    if !kernel_roundtrip() {
        return false;
    }
    if !el0_roundtrip() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "fs: ok");
    true
}

#[cfg(test)]
#[test_case]
fn vfs_memfs_create_write_read_close() {
    assert!(kernel_roundtrip());
    assert!(CREATE_OK.load(Ordering::SeqCst));
    assert!(WRITE_OK.load(Ordering::SeqCst));
    assert!(READ_OK.load(Ordering::SeqCst));
    reset();
    assert_eq!(open(KERNEL_PATH), Err(FsError::Missing));
    assert_eq!(create("nope"), Err(FsError::BadPath));
    assert_eq!(create("/bad/path"), Err(FsError::BadPath));
}

#[cfg(test)]
#[test_case]
fn vfs_open_missing_is_err() {
    reset();
    assert_eq!(open("/missing"), Err(FsError::Missing));
    assert_eq!(close(1), Err(FsError::BadHandle));
}

#[cfg(test)]
#[test_case]
fn el0_fs_svc_roundtrip() {
    assert!(paging::mmu_enabled());
    reset();
    assert!(
        el0_roundtrip(),
        "EL0 must SVC create/write/close/open/read/close"
    );
    assert!(!el0::is_active());
    assert_eq!(syscall::last_fs_read(), EL0_BYTES.len() as u64);
}

#[cfg(test)]
#[test_case]
fn el0_fs_write_rejects_kernel_data() {
    assert!(paging::mmu_enabled());
    reset();
    assert!(
        syscall::fs_el0_reject_kernel_data(),
        "SYS_FS_WRITE must reject a kernel .data pointer"
    );
    assert!(!el0::is_active());
}

#[cfg(test)]
#[test_case]
fn vfs_paths_and_caps_are_documented() {
    assert!(valid_path(KERNEL_PATH));
    assert!(valid_path(EL0_PATH));
    assert!(!valid_path("/"));
    assert!(!valid_path("probe"));
    assert!(!valid_path("/Probe"));
    assert_eq!(PATH_MAX, 16);
    assert_eq!(FILE_MAX, 256);
}
