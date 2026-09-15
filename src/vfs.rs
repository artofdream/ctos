//! Thin VFS: memfs (A6 / ADR-027) + FAT16 (A7 / ADR-028, write ADR-050,
//! readdir ADR-056, delete ADR-057) + prefix mounts (ADR-058).
//!
//! One `open` story. A small mount table routes path **prefixes** to a
//! backend (`/mem` + A6 probe names → memfs; `/` → FAT16). Not Linux VFS.
//! Not POSIX `mount` / `unlink` / `getdents`. Not app hosting.

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
pub const FILE_MAX: usize = 1024;
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
    /// Kept for ADR-028-era callers; FAT write is ADR-050 (no longer returned).
    #[allow(dead_code)]
    ReadOnly,
}

/// Thin VFS. memfs and FAT16 implement the same five calls.
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

impl MemFs {
    /// Remove a named buffer and drop handles that pointed at it.
    fn unlink(&mut self, path: &str) -> Result<(), FsError> {
        if !valid_path(path) {
            return Err(FsError::BadPath);
        }
        let slot = self.find_name(path).ok_or(FsError::Missing)?;
        self.files[slot] = None;
        for h in self.handles.iter_mut() {
            if h.used && h.file == slot {
                *h = Handle::empty();
            }
        }
        Ok(())
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

/// FAT fds are tagged so the router does not invent a second `open`.
const FAT_FD_TAG: u32 = 0x100;

/// Which backend owns a path prefix (ADR-058). Not a Linux `vfsmount`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    MemFs,
    Fat16,
}

/// One prefix → backend entry. Longest prefix wins.
#[derive(Clone, Copy, Debug)]
pub struct Mount {
    pub prefix: &'static str,
    pub backend: Backend,
}

/// Static first-cut mount table. `/mem` is the documented memfs home;
/// A6 probe names stay memfs so `/kprobe` / `/eprobe` / `/mwprobe` habits
/// hold; `/` routes everything else (incl. `/probe`, `/hello`) to FAT16.
pub const MOUNTS: &[Mount] = &[
    Mount {
        prefix: "/kprobe",
        backend: Backend::MemFs,
    },
    Mount {
        prefix: "/eprobe",
        backend: Backend::MemFs,
    },
    Mount {
        prefix: "/mwprobe",
        backend: Backend::MemFs,
    },
    Mount {
        prefix: "/mem",
        backend: Backend::MemFs,
    },
    Mount {
        prefix: "/",
        backend: Backend::Fat16,
    },
];

fn is_fat_fd(fd: u32) -> bool {
    fd & FAT_FD_TAG != 0
}

fn fat_inner(fd: u32) -> u32 {
    fd & !FAT_FD_TAG
}

fn backend_label(b: Backend) -> &'static str {
    match b {
        Backend::MemFs => "memfs",
        Backend::Fat16 => "fat",
    }
}

/// Longest-prefix mount lookup. Flat path grammar unchanged (`valid_path`).
pub fn resolve(path: &str) -> Result<Backend, FsError> {
    if !valid_path(path) {
        return Err(FsError::BadPath);
    }
    let mut best: Option<&Mount> = None;
    for m in MOUNTS {
        if path.starts_with(m.prefix)
            && best.map_or(true, |b| m.prefix.len() > b.prefix.len())
        {
            best = Some(m);
        }
    }
    Ok(best.expect("root mount `/` always matches").backend)
}

fn emit_mount_table() -> bool {
    if resolve("/probe") != Ok(Backend::Fat16) {
        return false;
    }
    if resolve("/hello") != Ok(Backend::Fat16) {
        return false;
    }
    if resolve("/kprobe") != Ok(Backend::MemFs) {
        return false;
    }
    if resolve("/eprobe") != Ok(Backend::MemFs) {
        return false;
    }
    if resolve("/memx") != Ok(Backend::MemFs) {
        return false;
    }
    let mut w = uart::raw();
    for m in MOUNTS {
        let _ = writeln!(
            w,
            "vfs: mount {} -> {}",
            m.prefix,
            backend_label(m.backend)
        );
    }
    let _ = writeln!(w, "vfs: mounts");
    true
}

pub fn create(path: &str) -> Result<u32, FsError> {
    match resolve(path)? {
        Backend::MemFs => FS.lock().create(path),
        Backend::Fat16 => crate::fat::create(path).map(|fd| fd | FAT_FD_TAG),
    }
}

pub fn open(path: &str) -> Result<u32, FsError> {
    match resolve(path)? {
        Backend::MemFs => FS.lock().open(path),
        Backend::Fat16 => crate::fat::open(path).map(|fd| fd | FAT_FD_TAG),
    }
}

pub fn read(fd: u32, buf: &mut [u8]) -> Result<usize, FsError> {
    if is_fat_fd(fd) {
        crate::fat::read(fat_inner(fd), buf)
    } else {
        FS.lock().read(fd, buf)
    }
}

pub fn write(fd: u32, buf: &[u8]) -> Result<usize, FsError> {
    if is_fat_fd(fd) {
        crate::fat::write(fat_inner(fd), buf)
    } else {
        FS.lock().write(fd, buf)
    }
}

pub fn close(fd: u32) -> Result<(), FsError> {
    if is_fat_fd(fd) {
        crate::fat::close(fat_inner(fd))
    } else {
        FS.lock().close(fd)
    }
}

/// List FAT16 root names as thin-VFS paths (`/probe`, …). Same entry point
/// story as `open`/`read`/`write` — not a second FS. Not POSIX `getdents`
/// / `opendir`. memfs has no directory tree this mile. Routed at the FAT
/// mount (`/`); not a union listing.
pub fn readdir(out: &mut [[u8; PATH_MAX]], cap: usize) -> Result<usize, FsError> {
    crate::fat::readdir(out, cap)
}

/// Delete a path on the mount that owns its prefix. Not POSIX `unlink`.
pub fn unlink(path: &str) -> Result<(), FsError> {
    match resolve(path)? {
        Backend::MemFs => FS.lock().unlink(path),
        Backend::Fat16 => crate::fat::unlink(path),
    }
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

/// Serial proof: prefix mount table + kernel + EL0 create/write/read/close.
/// FAT volume probes stay in `fat::observe_probe`.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() {
        return false;
    }
    if !emit_mount_table() {
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
    assert_eq!(FILE_MAX, 1024);
}

#[cfg(test)]
#[test_case]
fn vfs_fat_probe_is_same_open() {
    assert_eq!(open("/probe").map(|_| true), Ok(true));
    assert_eq!(create("/probe"), Err(FsError::Exists));
}

#[cfg(test)]
#[test_case]
fn vfs_prefix_mounts_route_backends() {
    assert_eq!(resolve("/probe"), Ok(Backend::Fat16));
    assert_eq!(resolve("/hello"), Ok(Backend::Fat16));
    assert_eq!(resolve("/fwr"), Ok(Backend::Fat16));
    assert_eq!(resolve("/kprobe"), Ok(Backend::MemFs));
    assert_eq!(resolve("/eprobe"), Ok(Backend::MemFs));
    assert_eq!(resolve("/mwprobe"), Ok(Backend::MemFs));
    assert_eq!(resolve("/memx"), Ok(Backend::MemFs));
    assert_eq!(resolve("/missing"), Ok(Backend::Fat16));
    assert_eq!(resolve("nope"), Err(FsError::BadPath));
    assert_eq!(resolve("/bad/path"), Err(FsError::BadPath));
    // Longest prefix: `/mem` wins over `/` for `/mem…`.
    assert!(MOUNTS.iter().any(|m| m.prefix == "/mem" && m.backend == Backend::MemFs));
    assert!(MOUNTS.iter().any(|m| m.prefix == "/" && m.backend == Backend::Fat16));
}

#[cfg(test)]
#[test_case]
fn vfs_memfs_mount_create_stays_off_fat() {
    reset();
    let fd = create(KERNEL_PATH).expect("memfs create on /kprobe mount");
    assert!(!is_fat_fd(fd));
    assert_eq!(write(fd, KERNEL_BYTES).ok(), Some(KERNEL_BYTES.len()));
    assert_eq!(close(fd), Ok(()));
    // Same name must not appear as a FAT root file.
    assert!(!crate::fat::has_name(KERNEL_PATH));
    let fd = open(KERNEL_PATH).expect("memfs open");
    let mut buf = [0u8; 16];
    let n = read(fd, &mut buf).expect("memfs read");
    assert_eq!(&buf[..n], KERNEL_BYTES);
    assert_eq!(close(fd), Ok(()));
    assert_eq!(unlink(KERNEL_PATH), Ok(()));
    assert_eq!(open(KERNEL_PATH), Err(FsError::Missing));
}
