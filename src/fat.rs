//! FAT16 on virtio-blk, behind the thin VFS (Track A / A7 / ADR-028).
//!
//! Read-only root file `/probe` (8.3 `PROBE`). Not FAT32. Not xv6.
//! Not POSIX. Host image is `scripts/mkfat16.py`.

use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

use crate::uart;
use crate::vfs::{self, FsError, VfsOps, FILE_MAX, PATH_MAX};
use crate::virtio;

const SECTOR: usize = 512;
const MAX_HANDLES: usize = 4;
const PROBE_PATH: &str = "/probe";
const PROBE_BYTES: &[u8] = b"fat-hi";

#[derive(Clone, Copy)]
struct FatHandle {
    used: bool,
    cluster: u16,
    size: u32,
    off: u32,
}

impl FatHandle {
    const fn empty() -> Self {
        Self {
            used: false,
            cluster: 0,
            size: 0,
            off: 0,
        }
    }
}

struct Fat16 {
    mounted: bool,
    bps: u16,
    spc: u8,
    reserved: u16,
    root_ents: u16,
    first_data: u32,
    root_sec: u32,
    handles: [FatHandle; MAX_HANDLES],
}

impl Fat16 {
    const fn empty() -> Self {
        Self {
            mounted: false,
            bps: 0,
            spc: 0,
            reserved: 0,
            root_ents: 0,
            first_data: 0,
            root_sec: 0,
            handles: [
                FatHandle::empty(),
                FatHandle::empty(),
                FatHandle::empty(),
                FatHandle::empty(),
            ],
        }
    }

    fn read_sector(&self, lba: u32, buf: &mut [u8; SECTOR]) -> bool {
        virtio::read_sector(lba as u64, buf)
    }

    fn fat_entry(&self, cluster: u16) -> Option<u16> {
        if cluster < 2 {
            return None;
        }
        let off = (cluster as u32) * 2;
        let sec = self.reserved as u32 + off / self.bps as u32;
        let ent = (off % self.bps as u32) as usize;
        let mut buf = [0u8; SECTOR];
        if !self.read_sector(sec, &mut buf) {
            return None;
        }
        Some(u16::from_le_bytes([buf[ent], buf[ent + 1]]))
    }

    fn cluster_lba(&self, cluster: u16) -> u32 {
        self.first_data + (cluster as u32 - 2) * self.spc as u32
    }

    fn lookup(&self, name83: &[u8; 11]) -> Option<(u16, u32)> {
        if !self.mounted {
            return None;
        }
        let root_secs = (self.root_ents as u32 * 32 + self.bps as u32 - 1) / self.bps as u32;
        let mut buf = [0u8; SECTOR];
        for s in 0..root_secs {
            if !self.read_sector(self.root_sec + s, &mut buf) {
                return None;
            }
            for i in 0..(self.bps as usize / 32) {
                let e = &buf[i * 32..i * 32 + 32];
                if e[0] == 0 {
                    return None;
                }
                if e[0] == 0xE5 {
                    continue;
                }
                let attr = e[11];
                if attr & 0x08 != 0 || attr & 0x0F == 0x0F || attr & 0x10 != 0 {
                    continue;
                }
                if &e[0..11] == name83 {
                    let cluster = u16::from_le_bytes([e[26], e[27]]);
                    let size = u32::from_le_bytes([e[28], e[29], e[30], e[31]]);
                    return Some((cluster, size));
                }
            }
        }
        None
    }

    fn read_at(&self, start: u16, size: u32, off: u32, out: &mut [u8]) -> Result<usize, FsError> {
        if off > size {
            return Err(FsError::BadHandle);
        }
        let remain = (size - off) as usize;
        let n = core::cmp::min(out.len(), remain);
        if n == 0 {
            return Ok(0);
        }
        let bps = self.bps as u32;
        let clus_bytes = bps * self.spc as u32;
        let mut cluster = start;
        let mut skipped = 0u32;
        while skipped + clus_bytes <= off {
            cluster = self.fat_entry(cluster).ok_or(FsError::BadHandle)?;
            if cluster < 2 || cluster >= 0xFFF8 {
                return Err(FsError::BadHandle);
            }
            skipped += clus_bytes;
        }
        let mut done = 0;
        let mut cur_off = off - skipped;
        while done < n {
            if cluster < 2 || cluster >= 0xFFF8 {
                break;
            }
            let mut sec = [0u8; SECTOR];
            if !self.read_sector(self.cluster_lba(cluster), &mut sec) {
                return Err(FsError::BadHandle);
            }
            let start = cur_off as usize;
            let take = core::cmp::min(n - done, SECTOR - start);
            out[done..done + take].copy_from_slice(&sec[start..start + take]);
            done += take;
            cur_off = 0;
            if done < n {
                cluster = self.fat_entry(cluster).ok_or(FsError::BadHandle)?;
            }
        }
        Ok(done)
    }

    fn alloc_handle(&mut self, cluster: u16, size: u32) -> Result<u32, FsError> {
        for (i, h) in self.handles.iter_mut().enumerate() {
            if !h.used {
                *h = FatHandle {
                    used: true,
                    cluster,
                    size,
                    off: 0,
                };
                return Ok((i as u32) + 1);
            }
        }
        Err(FsError::Full)
    }

    fn handle_mut(&mut self, fd: u32) -> Result<&mut FatHandle, FsError> {
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

impl VfsOps for Fat16 {
    fn create(&mut self, _path: &str) -> Result<u32, FsError> {
        Err(FsError::ReadOnly)
    }

    fn open(&mut self, path: &str) -> Result<u32, FsError> {
        let name = path_to_83(path).ok_or(FsError::BadPath)?;
        let (cluster, size) = self.lookup(&name).ok_or(FsError::Missing)?;
        self.alloc_handle(cluster, size)
    }

    fn read(&mut self, fd: u32, buf: &mut [u8]) -> Result<usize, FsError> {
        let (cluster, size, off) = {
            let h = self.handle_mut(fd)?;
            (h.cluster, h.size, h.off)
        };
        let cap = core::cmp::min(buf.len(), FILE_MAX);
        let n = self.read_at(cluster, size, off, &mut buf[..cap])?;
        self.handle_mut(fd)?.off = off + n as u32;
        Ok(n)
    }

    fn write(&mut self, _fd: u32, _buf: &[u8]) -> Result<usize, FsError> {
        Err(FsError::ReadOnly)
    }

    fn close(&mut self, fd: u32) -> Result<(), FsError> {
        let h = self.handle_mut(fd)?;
        *h = FatHandle::empty();
        Ok(())
    }
}

static FAT: Mutex<Fat16> = Mutex::new(Fat16::empty());
static MOUNT_OK: AtomicBool = AtomicBool::new(false);
static READ_OK: AtomicBool = AtomicBool::new(false);

fn path_to_83(path: &str) -> Option<[u8; 11]> {
    if !vfs::valid_path(path) {
        return None;
    }
    let name = &path.as_bytes()[1..];
    if name.is_empty() || name.len() > 8 {
        return None;
    }
    let mut out = [b' '; 11];
    for (i, &c) in name.iter().enumerate() {
        out[i] = c.to_ascii_uppercase();
    }
    Some(out)
}

fn mount_from_boot(boot: &[u8; SECTOR]) -> Option<Fat16> {
    if boot[510] != 0x55 || boot[511] != 0xAA {
        return None;
    }
    let bps = u16::from_le_bytes([boot[11], boot[12]]);
    let spc = boot[13];
    let reserved = u16::from_le_bytes([boot[14], boot[15]]);
    let fats = boot[16];
    let root_ents = u16::from_le_bytes([boot[17], boot[18]]);
    let tot16 = u16::from_le_bytes([boot[19], boot[20]]);
    let fat_sz = u16::from_le_bytes([boot[22], boot[23]]);
    let tot32 = u32::from_le_bytes([boot[32], boot[33], boot[34], boot[35]]);
    if bps != 512 || spc == 0 || reserved == 0 || fats == 0 || fat_sz == 0 || root_ents == 0 {
        return None;
    }
    let total = if tot16 != 0 { tot16 as u32 } else { tot32 };
    if total == 0 {
        return None;
    }
    let root_secs = (root_ents as u32 * 32 + bps as u32 - 1) / bps as u32;
    let first_data = reserved as u32 + fats as u32 * fat_sz as u32 + root_secs;
    if first_data >= total {
        return None;
    }
    let clusters = (total - first_data) / spc as u32;
    // Microsoft: 4085..65524 inclusive is FAT16.
    if clusters < 4085 || clusters >= 65525 {
        return None;
    }
    Some(Fat16 {
        mounted: true,
        bps,
        spc,
        reserved,
        root_ents,
        first_data,
        root_sec: reserved as u32 + fats as u32 * fat_sz as u32,
        handles: [
            FatHandle::empty(),
            FatHandle::empty(),
            FatHandle::empty(),
            FatHandle::empty(),
        ],
    })
}

pub fn init() {
    MOUNT_OK.store(false, Ordering::SeqCst);
    READ_OK.store(false, Ordering::SeqCst);
    let mut fat = FAT.lock();
    *fat = Fat16::empty();
    if !virtio::ready() {
        return;
    }
    let mut boot = [0u8; SECTOR];
    if !virtio::read_sector(0, &mut boot) {
        return;
    }
    let Some(mounted) = mount_from_boot(&boot) else {
        return;
    };
    *fat = mounted;
    MOUNT_OK.store(true, Ordering::SeqCst);
}

pub fn mounted() -> bool {
    MOUNT_OK.load(Ordering::SeqCst)
}

pub fn has_name(path: &str) -> bool {
    let Some(name) = path_to_83(path) else {
        return false;
    };
    FAT.lock().lookup(&name).is_some()
}

pub fn open(path: &str) -> Result<u32, FsError> {
    FAT.lock().open(path)
}

pub fn read(fd: u32, buf: &mut [u8]) -> Result<usize, FsError> {
    FAT.lock().read(fd, buf)
}

pub fn write(fd: u32, buf: &[u8]) -> Result<usize, FsError> {
    FAT.lock().write(fd, buf)
}

pub fn close(fd: u32) -> Result<(), FsError> {
    FAT.lock().close(fd)
}

fn vfs_probe_read() -> bool {
    READ_OK.store(false, Ordering::SeqCst);
    let Ok(fd) = vfs::open(PROBE_PATH) else {
        return false;
    };
    let mut buf = [0u8; PATH_MAX];
    let Ok(n) = vfs::read(fd, &mut buf) else {
        let _ = vfs::close(fd);
        return false;
    };
    let _ = vfs::close(fd);
    if n != PROBE_BYTES.len() || &buf[..n] != PROBE_BYTES {
        return false;
    }
    uart::write_str_raw("fat: read\n");
    READ_OK.store(true, Ordering::SeqCst);
    true
}

/// Serial proof: mount FAT16 and VFS-read `/probe`. Not a second open story.
#[allow(dead_code)]
pub fn observe_probe() -> bool {
    if !mounted() {
        return false;
    }
    uart::write_str_raw("fat: mount\n");
    if !vfs_probe_read() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "fat: ok");
    true
}

#[cfg(test)]
#[test_case]
fn fat16_vfs_open_probe() {
    assert!(mounted(), "FAT16 must mount from the attached image");
    assert!(has_name(PROBE_PATH));
    assert!(vfs_probe_read());
    assert!(READ_OK.load(Ordering::SeqCst));
}

#[cfg(test)]
#[test_case]
fn fat16_missing_and_readonly() {
    assert!(mounted());
    assert_eq!(open("/missing"), Err(FsError::Missing));
    assert_eq!(open("/probe-too-long-name"), Err(FsError::BadPath));
    let fd = open(PROBE_PATH).expect("PROBE");
    assert_eq!(write(fd, b"x"), Err(FsError::ReadOnly));
    assert_eq!(close(fd), Ok(()));
    assert_eq!(FAT.lock().create("/nope"), Err(FsError::ReadOnly));
}

#[cfg(test)]
#[test_case]
fn fat16_path_maps_8_3() {
    assert_eq!(path_to_83("/probe"), Some(*b"PROBE      "));
    assert_eq!(path_to_83("/kprobe"), Some(*b"KPROBE     "));
    assert!(path_to_83("/toolong1").is_none());
    assert!(path_to_83("probe").is_none());
}
